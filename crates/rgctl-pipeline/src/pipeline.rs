//! Parallel processing pipeline
//!
//! Task 1.6.2: Parallel file parsing with rayon

use crate::stream::{DEFAULT_STREAM_CHANNEL_CAPACITY, stream_into_graph};
use indicatif::{ProgressBar, ProgressStyle};
use rgctl_error::Result;
use rgctl_extraction::discovery::{DiscoveryConfig, FileDiscoverer};
use rgctl_extraction::{Extractor, GraphBuilder};
use rgctl_graph::code_graph::CodeGraph;
use rgctl_graph::code_index::CodeIndex;
use rgctl_graph::content_store::ContentStore;
use rgctl_graph::schema::{Edge, Node};
use rgctl_graph::write_columnar_from_spill;
use rgctl_registry::LanguageRegistry;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Options for the processing pipeline.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// File discovery configuration
    pub discovery: DiscoveryConfig,
    /// Show progress bar during processing
    pub show_progress: bool,
    /// Optional thread count for parallel extraction (defaults to rayon pool size)
    pub thread_count: Option<usize>,
    /// Batch size for parallel file processing
    pub batch_size: usize,
    /// Max in-flight extractions between parallel workers and graph merge
    pub stream_channel_capacity: usize,
    /// Materialize `Symbol.fields` as Variable graph nodes (CPG / `--with-cfg`).
    pub materialize_fields: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            discovery: DiscoveryConfig::default(),
            show_progress: true,
            thread_count: None,
            batch_size: 64,
            stream_channel_capacity: DEFAULT_STREAM_CHANNEL_CAPACITY,
            materialize_fields: false,
        }
    }
}

/// Statistics from a pipeline run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PipelineStats {
    /// Files discovered
    pub files_discovered: usize,
    /// Files successfully processed
    pub files_processed: usize,
    /// Files that failed extraction
    pub files_failed: usize,
    /// Nodes created in the graph
    pub nodes_created: usize,
    /// Edges created in the graph
    pub edges_created: usize,
    /// Total processing duration
    pub duration: Duration,
    /// Time spent in parallel file extraction (tree-sitter)
    pub extract_duration: Duration,
    /// Time spent merging extractions into the graph
    pub graph_build_duration: Duration,
}

/// End-to-end repository processing pipeline.
pub struct ProcessingPipeline {
    registry: Arc<LanguageRegistry>,
    config: PipelineConfig,
}

impl ProcessingPipeline {
    /// Create a pipeline with default configuration.
    pub fn new(registry: Arc<LanguageRegistry>) -> Self {
        Self {
            registry,
            config: PipelineConfig::default(),
        }
    }

    /// Create a pipeline with custom configuration.
    pub fn with_config(registry: Arc<LanguageRegistry>, config: PipelineConfig) -> Self {
        Self { registry, config }
    }

    /// Discover, extract, and build a graph for a repository.
    pub fn process_repository(&self, root: &Path) -> Result<(CodeGraph, PipelineStats)> {
        let start = Instant::now();
        let (nodes, edges, mut stats) = self.extract_repository(root)?;
        let load_start = Instant::now();
        let mut graph = CodeGraph::new();
        graph.load(nodes, edges)?;
        stats.graph_build_duration += load_start.elapsed();
        stats.duration = start.elapsed();
        Ok((graph, stats))
    }

    /// Discover, extract, and write a columnar snapshot without building [`CodeGraph`].
    ///
    /// Spills nodes/edges to disk during extract, then externally sorts and compiles
    /// the columnar snapshot (no full `Vec<Node>` / `Vec<Edge>` residency).
    ///
    /// `source_root` is scanned for files; `artifact_root` receives spill/content/code
    /// artifacts (defaults to `source_root` when unset).
    pub fn process_repository_to_snapshot(
        &self,
        source_root: &Path,
        snapshot_path: &Path,
        artifact_root: Option<&Path>,
    ) -> Result<(PipelineStats, String)> {
        let start = Instant::now();
        let discoverer =
            FileDiscoverer::with_config(Arc::clone(&self.registry), self.config.discovery.clone());
        let files = discoverer.discover(source_root)?;
        let files_discovered = files.len();

        let progress = self.make_progress(files_discovered);
        let extractor = Extractor::new(Arc::clone(&self.registry));
        let extract_start = Instant::now();

        let store = artifact_root.unwrap_or(source_root);
        let spill_dir = rgctl_graph::paths::artifact_path(store, "spill");
        if spill_dir.exists() {
            std::fs::remove_dir_all(&spill_dir)?;
        }
        std::fs::create_dir_all(&spill_dir)?;
        let mut builder = GraphBuilder::with_spill(&spill_dir)?;
        builder.set_materialize_fields(self.config.materialize_fields);
        builder.set_code_index(CodeIndex::load(CodeIndex::default_cache_path(store))?);
        builder.set_content_store(ContentStore::load(ContentStore::default_path(store))?);

        let progress_for_stream = progress.clone();
        let (stream_stats, tails) = stream_into_graph(
            self.config.thread_count,
            &extractor,
            Arc::clone(&self.registry),
            &files,
            self.config.stream_channel_capacity,
            &mut builder,
            move || {
                if let Some(pb) = &progress_for_stream {
                    pb.inc(1);
                }
            },
        )?;
        let extract_duration = extract_start.elapsed();

        if let Some(pb) = progress {
            pb.finish_with_message("done");
        }

        let files_processed = stream_stats.files_processed;
        let files_failed = stream_stats.extraction_failures.len();

        let graph_start = Instant::now();
        let index_start = Instant::now();
        builder.build_resolution_indexes();
        let index_elapsed = index_start.elapsed();
        let pass2_start = Instant::now();
        extractor.populate_pass2(&tails, &mut builder)?;
        let pass2_elapsed = pass2_start.elapsed();
        builder.log_resolution_stats();
        let nodes_created = builder.node_count();
        let edges_created = builder.edge_count();
        let content_store = builder.take_content_store();
        let code_index = builder.take_code_index();
        let spill_start = Instant::now();
        let finished = builder.finish_spill()?;
        let digest = write_columnar_from_spill(finished, snapshot_path)?;
        let spill_elapsed = spill_start.elapsed();
        tracing::info!(
            resolution_index_secs = index_elapsed.as_secs_f64(),
            pass2_relation_resolution_secs = pass2_elapsed.as_secs_f64(),
            spill_and_columnar_secs = spill_elapsed.as_secs_f64(),
            "graph build sub-phase timings"
        );
        if let Some(store) = content_store {
            store.save()?;
        }
        if let Some(index) = code_index {
            index.save()?;
        }
        let graph_build_duration = graph_start.elapsed();

        Ok((
            PipelineStats {
                files_discovered,
                files_processed,
                files_failed,
                nodes_created,
                edges_created,
                duration: start.elapsed(),
                extract_duration,
                graph_build_duration,
            },
            digest,
        ))
    }

    fn make_progress(&self, files_discovered: usize) -> Option<ProgressBar> {
        if self.config.show_progress && files_discovered > 0 {
            let pb = ProgressBar::new(files_discovered as u64);
            pb.set_style(
                ProgressStyle::with_template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
                )
                .unwrap()
                .progress_chars("#>-"),
            );
            pb.set_message("extracting");
            Some(pb)
        } else {
            None
        }
    }

    fn extract_repository(&self, root: &Path) -> Result<(Vec<Node>, Vec<Edge>, PipelineStats)> {
        let start = Instant::now();
        let discoverer =
            FileDiscoverer::with_config(Arc::clone(&self.registry), self.config.discovery.clone());
        let files = discoverer.discover(root)?;
        let files_discovered = files.len();

        let progress = self.make_progress(files_discovered);
        let extractor = Extractor::new(Arc::clone(&self.registry));
        let extract_start = Instant::now();
        let mut builder = GraphBuilder::new();
        builder.set_materialize_fields(self.config.materialize_fields);
        builder.set_code_index(CodeIndex::load(CodeIndex::default_cache_path(root))?);
        builder.set_content_store(ContentStore::load(ContentStore::default_path(root))?);
        let progress_for_stream = progress.clone();
        let (stream_stats, tails) = stream_into_graph(
            self.config.thread_count,
            &extractor,
            Arc::clone(&self.registry),
            &files,
            self.config.stream_channel_capacity,
            &mut builder,
            move || {
                if let Some(pb) = &progress_for_stream {
                    pb.inc(1);
                }
            },
        )?;
        let extract_duration = extract_start.elapsed();

        if let Some(pb) = progress {
            pb.finish_with_message("done");
        }

        let files_processed = stream_stats.files_processed;
        let files_failed = stream_stats.extraction_failures.len();

        let graph_start = Instant::now();
        builder.build_resolution_indexes();
        extractor.populate_pass2(&tails, &mut builder)?;
        if let Some(store) = builder.take_content_store() {
            store.save()?;
        }
        if let Some(index) = builder.take_code_index() {
            index.save()?;
        }
        let (nodes, edges): (Vec<Node>, Vec<Edge>) = builder.into_graph();
        let graph_build_duration = graph_start.elapsed();

        let nodes_created = nodes.len();
        let edges_created = edges.len();
        Ok((
            nodes,
            edges,
            PipelineStats {
                files_discovered,
                files_processed,
                files_failed,
                nodes_created,
                edges_created,
                duration: start.elapsed(),
                extract_duration,
                graph_build_duration,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_parsing() {
        let temp = TempDir::new().unwrap();
        for i in 0..10 {
            fs::write(
                temp.path().join(format!("file{i}.rs")),
                format!("fn func{i}() {{}}\n"),
            )
            .unwrap();
        }

        let config = PipelineConfig {
            show_progress: false,
            ..PipelineConfig::default()
        };
        let pipeline = ProcessingPipeline::with_config(
            Arc::new(rgctl_languages::default_registry()),
            config,
        );
        let (graph, stats) = pipeline.process_repository(temp.path()).unwrap();

        assert_eq!(stats.files_processed, 10);
        assert!(graph.node_count() > 10);
    }
}
