//! Symbol and relationship extraction

use crate::discovery::{DiscoveryConfig, FileDiscoverer};
use crate::graph_builder::GraphBuilder;
use crate::usage_detector::{ConfigUsage, ConfigUsageDetector};
use rgctl_error::{Error, Result};
use rgctl_plugin_api::{ConfigKey, Relation, Symbol};
use rgctl_registry::LanguageRegistry;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Extracts symbols and relationships from a single file.
pub struct Extractor {
    registry: Arc<LanguageRegistry>,
}

/// Result of extracting a single file.
#[derive(Debug, Default, Clone)]
pub struct FileExtraction {
    /// Path to the source file
    pub path: PathBuf,
    /// Extracted code symbols
    pub symbols: Vec<Symbol>,
    /// Extracted symbol relations
    pub relations: Vec<Relation>,
    /// Extracted configuration keys
    pub config_keys: Vec<ConfigKey>,
    /// Detected configuration usages in source
    pub config_usages: Vec<ConfigUsage>,
    /// Cached source bytes (avoids re-reading file during graph population)
    pub source: Vec<u8>,
    /// Out-of-line content blobs keyed by Blake3 hex (`body_ref`).
    pub content_blobs: HashMap<String, String>,
}

/// Lightweight remainder after pass-1 graph population (symbols committed; source dropped).
#[derive(Debug, Default)]
pub struct ExtractionTail {
    /// Relations resolved in pass 2
    pub relations: Vec<Relation>,
    /// Config usages linked in pass 2
    pub config_usages: Vec<ConfigUsage>,
}

#[derive(Debug, Default, Clone, Copy)]
struct Pass1Profile {
    symbol_processing: Duration,
    config_key_processing: Duration,
}

#[derive(Debug, Default, Clone, Copy)]
struct Pass2Profile {
    relation_resolution: Duration,
    config_usage_resolution: Duration,
}

impl Extractor {
    /// Create a new extractor backed by the given registry.
    pub fn new(registry: Arc<LanguageRegistry>) -> Self {
        Self { registry }
    }

    /// Discover and extract all processable files under `root`.
    pub fn extract_repository(
        &self,
        root: &Path,
        discovery: &DiscoveryConfig,
    ) -> Result<Vec<FileExtraction>> {
        let discoverer = FileDiscoverer::with_config(Arc::clone(&self.registry), discovery.clone());
        let files = discoverer.discover(root)?;
        Ok(files
            .iter()
            .filter_map(|path| match self.extract_file(path) {
                Ok(extraction) => Some(extraction),
                Err(err) => {
                    tracing::warn!("Failed to extract {}: {}", path.display(), err);
                    None
                }
            })
            .collect())
    }

    /// Extract symbols, relations, and config references from one file.
    pub fn extract_file(&self, path: &Path) -> Result<FileExtraction> {
        let source = std::fs::read(path)?;

        if let Ok(plugin) = self.registry.get_plugin_for_file(path) {
            let extracted = plugin.extract_all(path, &source)?;
            let config_usages = ConfigUsageDetector::detect(plugin.language_id(), &source, path);
            return Ok(FileExtraction {
                path: path.to_path_buf(),
                symbols: extracted.symbols,
                relations: extracted.relations,
                config_keys: Vec::new(),
                config_usages,
                source,
                content_blobs: extracted.content_blobs,
            });
        }

        if let Ok(plugin) = self.registry.get_config_plugin_for_file(path) {
            let config_keys = plugin.extract_config_keys(path, &source)?;
            return Ok(FileExtraction {
                path: path.to_path_buf(),
                symbols: Vec::new(),
                relations: Vec::new(),
                config_keys,
                config_usages: Vec::new(),
                source,
                content_blobs: HashMap::new(),
            });
        }

        Err(Error::UnsupportedLanguage(
            path.to_string_lossy().to_string(),
        ))
    }

    /// Pass 1 for one file: add symbols/config keys, then drop source from memory.
    pub fn populate_pass1(
        &self,
        extraction: &mut FileExtraction,
        builder: &mut GraphBuilder,
    ) -> Result<ExtractionTail> {
        let (tail, _profile) = self.populate_pass1_profiled(extraction, builder)?;
        Ok(tail)
    }

    fn populate_pass1_profiled(
        &self,
        extraction: &mut FileExtraction,
        builder: &mut GraphBuilder,
    ) -> Result<(ExtractionTail, Pass1Profile)> {
        use std::time::Instant;

        let mut profile = Pass1Profile::default();
        let file_id = builder.ensure_file_node_with_source(
            &extraction.path,
            (!extraction.source.is_empty()).then_some(extraction.source.as_slice()),
        );
        builder.merge_content_blobs(&extraction.content_blobs);
        let source = (!extraction.source.is_empty()).then_some(extraction.source.as_slice());
        let line_offsets = source.map(line_start_offsets);

        if !extraction.symbols.is_empty() {
            let symbol_start = Instant::now();
            for symbol in &extraction.symbols {
                let body = source.and_then(|bytes| {
                    let offsets = line_offsets.as_ref()?;
                    symbol_body_from_source(bytes, offsets, symbol)
                });
                if let Some(body) = body.as_deref() {
                    builder.add_symbol_with_body(symbol, file_id, Some(body));
                } else {
                    builder.add_symbol(symbol, file_id);
                }
            }
            profile.symbol_processing += symbol_start.elapsed();
        }

        if !extraction.config_keys.is_empty() {
            let config_key_start = Instant::now();
            for key in &extraction.config_keys {
                builder.add_config_key(key, file_id);
            }
            profile.config_key_processing += config_key_start.elapsed();
        }

        extraction.content_blobs.clear();
        extraction.source.clear();
        extraction.symbols.clear();
        extraction.config_keys.clear();

        Ok((
            ExtractionTail {
                relations: std::mem::take(&mut extraction.relations),
                config_usages: std::mem::take(&mut extraction.config_usages),
            },
            profile,
        ))
    }

    /// Pass 2: resolve relations and config usages (requires [`GraphBuilder::build_resolution_indexes`]).
    pub fn populate_pass2(
        &self,
        tails: &[ExtractionTail],
        builder: &mut GraphBuilder,
    ) -> Result<()> {
        let _ = self.populate_pass2_profiled(tails, builder)?;
        Ok(())
    }

    fn populate_pass2_profiled(
        &self,
        tails: &[ExtractionTail],
        builder: &mut GraphBuilder,
    ) -> Result<Pass2Profile> {
        use std::time::Instant;

        let mut profile = Pass2Profile::default();
        for tail in tails {
            if !tail.relations.is_empty() {
                let relation_start = Instant::now();
                for relation in &tail.relations {
                    builder.add_relation(relation)?;
                }
                profile.relation_resolution += relation_start.elapsed();
            }

            if !tail.config_usages.is_empty() {
                let config_usage_start = Instant::now();
                for usage in &tail.config_usages {
                    builder.link_config_usage(
                        &usage.file,
                        usage.line,
                        &usage.key,
                        usage.usage_type,
                    );
                }
                profile.config_usage_resolution += config_usage_start.elapsed();
            }
        }
        Ok(profile)
    }

    /// Merge extracted files into a graph builder.
    pub fn populate_graph(
        &self,
        extractions: &[FileExtraction],
        builder: &mut GraphBuilder,
    ) -> Result<()> {
        use std::time::Instant;
        use tracing::info;

        let total_start = Instant::now();
        let file_count = extractions.len();
        let total_symbols: usize = extractions.iter().map(|e| e.symbols.len()).sum();
        let total_relations: usize = extractions.iter().map(|e| e.relations.len()).sum();
        let total_config_keys: usize = extractions.iter().map(|e| e.config_keys.len()).sum();
        let total_config_usages: usize = extractions.iter().map(|e| e.config_usages.len()).sum();

        info!(
            file_count,
            total_symbols,
            total_relations,
            total_config_keys,
            total_config_usages,
            "populate_graph starting"
        );

        let file_io_time = std::time::Duration::ZERO;
        let mut symbol_time = std::time::Duration::ZERO;
        let mut relation_time = std::time::Duration::ZERO;
        let mut config_key_time = std::time::Duration::ZERO;
        let mut config_usage_time = std::time::Duration::ZERO;

        let mut tails = Vec::with_capacity(file_count);
        for extraction in extractions {
            let mut owned = extraction.clone();
            let (tail, pass1_profile) = self.populate_pass1_profiled(&mut owned, builder)?;
            symbol_time += pass1_profile.symbol_processing;
            config_key_time += pass1_profile.config_key_processing;
            tails.push(tail);
        }

        builder.build_resolution_indexes();

        let pass2_profile = self.populate_pass2_profiled(&tails, builder)?;
        relation_time += pass2_profile.relation_resolution;
        config_usage_time += pass2_profile.config_usage_resolution;

        let total_elapsed = total_start.elapsed();
        info!(
            elapsed_total_secs = total_elapsed.as_secs_f64(),
            file_io_secs = file_io_time.as_secs_f64(),
            symbol_processing_secs = symbol_time.as_secs_f64(),
            relation_resolution_secs = relation_time.as_secs_f64(),
            config_key_secs = config_key_time.as_secs_f64(),
            config_usage_secs = config_usage_time.as_secs_f64(),
            "populate_graph complete"
        );

        builder.log_resolution_stats();

        Ok(())
    }
}

fn line_start_offsets(source: &[u8]) -> Vec<usize> {
    let mut offsets = Vec::with_capacity(source.len() / 32 + 1);
    offsets.push(0);
    for (i, &b) in source.iter().enumerate() {
        if b == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

fn symbol_body_from_source(
    source: &[u8],
    line_offsets: &[usize],
    symbol: &Symbol,
) -> Option<String> {
    let start = symbol.location.start_line.saturating_sub(1);
    let end_line = symbol.location.end_line.max(symbol.location.start_line);
    if start >= line_offsets.len() {
        return None;
    }
    let start_byte = line_offsets[start];
    let end_byte = if end_line < line_offsets.len() {
        line_offsets[end_line]
    } else {
        source.len()
    };
    let slice = source.get(start_byte..end_byte)?;
    let text = std::str::from_utf8(slice).ok()?.trim_end_matches('\n');
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_extract_rust_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("main.rs");
        fs::write(&path, "fn hello() {}\nfn world() {}\n").unwrap();

        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(registry);
        let result = extractor.extract_file(&path).unwrap();

        assert_eq!(result.symbols.len(), 2);
        assert!(result.symbols.iter().any(|s| s.name == "hello"));
    }

    #[test]
    fn test_extract_yaml_config() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("config.yaml");
        fs::write(&path, "server:\n  port: 8080\n").unwrap();

        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(registry);
        let result = extractor.extract_file(&path).unwrap();

        assert!(
            result
                .config_keys
                .iter()
                .any(|k| k.key_path == "server.port")
        );
    }

    #[test]
    fn test_populate_graph() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("lib.rs");
        fs::write(&path, "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();

        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(registry);
        let extraction = extractor.extract_file(&path).unwrap();

        let mut builder = GraphBuilder::new();
        extractor
            .populate_graph(&[extraction], &mut builder)
            .unwrap();

        assert!(builder.node_count() >= 2);
        assert!(builder.edge_count() >= 2);
    }

    #[test]
    fn test_extract_markdown_headings_and_relations() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("docs/guide.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, "# Checkout Flow\n\n## Cart\n\n[ADR](./adr.md)\n").unwrap();
        let adr = temp.path().join("docs/adr.md");
        fs::write(&adr, "# Payments\n").unwrap();

        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(registry);
        let extraction = extractor.extract_file(&path).unwrap();

        let expected_qn = format!("{}#checkout-flow", path.to_string_lossy());
        assert!(
            extraction.symbols.iter().any(|s| s.name == "Checkout Flow"
                && s.qualified_name.as_deref() == Some(expected_qn.as_str())),
            "heading symbol"
        );
        assert!(
            extraction
                .relations
                .iter()
                .any(|r| { r.relation_type == rgctl_plugin_api::RelationType::Defines }),
            "Defines relation"
        );
        assert!(
            extraction
                .relations
                .iter()
                .any(|r| { r.relation_type == rgctl_plugin_api::RelationType::References }),
            "References relation"
        );
    }

    #[test]
    fn profiling_keeps_empty_buckets_zero() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("lib.rs");
        fs::write(&path, "pub fn add(a: i32, b: i32) -> i32 { a + b }\n").unwrap();

        let registry = Arc::new(rgctl_languages::default_registry());
        let extractor = Extractor::new(registry);
        let mut extraction = extractor.extract_file(&path).unwrap();
        let mut builder = GraphBuilder::new();

        let (tail, pass1) = extractor
            .populate_pass1_profiled(&mut extraction, &mut builder)
            .unwrap();
        assert_eq!(pass1.config_key_processing, Duration::ZERO);

        builder.build_resolution_indexes();
        let pass2 = extractor
            .populate_pass2_profiled(&[tail], &mut builder)
            .unwrap();
        assert_eq!(pass2.config_usage_resolution, Duration::ZERO);
    }
}
