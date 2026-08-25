//! Two-stage semantic retrieval: Hamming pre-filter + late structural fusion.

use crate::results::AnalysisResults;
use crate::semantic_embedder::{OnnxReloadOptions, embedder_for_index};
use crate::semantic_extract::tokenize_string_into;
use crate::semantic_search::{SemanticEntry, SemanticHit, SemanticIndex, hamming_top_k};
use rgctl_error::Result;
use rgctl_graph::backend::{GraphBackend, MemoryBackend};
use rgctl_graph::schema::EdgeType;
use rgctl_graph::{TokenBloom, keyword_overlap_score, satisfies_keyword_and};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

/// Default Hamming candidate pool size before late fusion.
pub const DEFAULT_CANDIDATE_POOL: usize = 256;

/// Configuration for late fusion re-ranking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SemanticFusionConfig {
    /// When false, return pure Hamming top-k.
    pub enabled: bool,
    /// Number of Hamming candidates to retrieve before fusion.
    pub candidate_pool: usize,
    /// Require every query keyword to match entry metadata tokens.
    pub keyword_and: bool,
    /// Weight for normalized Hamming similarity.
    pub w_semantic: f64,
    /// Weight for blast-radius impact score.
    pub w_blast: f64,
    /// Weight for PageRank centrality.
    pub w_centrality: f64,
    /// Weight for identifier / metadata token overlap.
    pub w_name: f64,
    /// Weight for eager token bloom overlap (body + declaration sketch).
    pub w_sketch: f64,
    /// Weight for sharing the name-hit community (graph-native).
    pub w_community: f64,
    /// Weight for sharing the name-hit package / path prefix.
    pub w_package: f64,
    /// Weight for callee-name overlap with the query (API-signature analog).
    pub w_callees: f64,
}

impl Default for SemanticFusionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            candidate_pool: DEFAULT_CANDIDATE_POOL,
            keyword_and: false,
            w_semantic: 0.35,
            w_blast: 0.25,
            w_centrality: 0.20,
            w_name: 0.05,
            w_sketch: 0.15,
            w_community: 0.08,
            w_package: 0.08,
            w_callees: 0.10,
        }
    }
}

/// One Hamming candidate enriched with fused ranking score.
#[derive(Debug, Clone, PartialEq)]
pub struct FusionCandidate {
    /// Row index in the semantic index.
    pub row: usize,
    /// Hamming distance (lower is better).
    pub hamming_distance: u32,
    /// Fused ranking score (higher is better).
    pub fused_score: f64,
    /// Indexed function metadata.
    pub entry: SemanticEntry,
}

/// Tokenize a query string into normalized keywords (length >= [`MIN_TOKEN_LEN`]).
pub fn query_keywords(query: &str) -> Vec<String> {
    let mut tokens = HashSet::new();
    tokenize_string_into(query, &mut tokens);
    let mut list: Vec<String> = tokens.into_iter().collect();
    list.sort_unstable();
    list
}

/// Collect lowercase metadata tokens for an indexed function.
pub fn entry_metadata_tokens(entry: &SemanticEntry) -> HashSet<String> {
    let mut tokens = HashSet::new();
    tokenize_string_into(&entry.name, &mut tokens);
    if let Some(qn) = &entry.qualified_name {
        tokenize_string_into(qn, &mut tokens);
    }
    tokens
}

/// True when every keyword matches metadata and/or an eager token bloom sketch.
pub fn keyword_and_matches(
    entry: &SemanticEntry,
    keywords: &[String],
    node_bloom: Option<&TokenBloom>,
) -> bool {
    if keywords.is_empty() {
        return true;
    }
    if let Some(bloom) = node_bloom {
        if satisfies_keyword_and(keywords, bloom) {
            return true;
        }
    }
    keyword_and_matches_metadata(entry, keywords)
}

/// Metadata-only keyword AND (name / qualified name tokens).
pub fn keyword_and_matches_metadata(entry: &SemanticEntry, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let tokens = entry_metadata_tokens(entry);
    let name_lower = entry.name.to_ascii_lowercase();
    let qname_lower = entry
        .qualified_name
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    keywords.iter().all(|keyword| {
        tokens.contains(keyword) || name_lower.contains(keyword) || qname_lower.contains(keyword)
    })
}

/// Fraction of query keywords matched in entry metadata.
pub fn name_overlap_score(entry: &SemanticEntry, keywords: &[String]) -> f64 {
    if keywords.is_empty() {
        return 0.0;
    }
    let tokens = entry_metadata_tokens(entry);
    let name_lower = entry.name.to_ascii_lowercase();
    let qname_lower = entry
        .qualified_name
        .as_deref()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    let matched = keywords
        .iter()
        .filter(|keyword| {
            tokens.contains(*keyword)
                || name_lower.contains(keyword.as_str())
                || qname_lower.contains(keyword.as_str())
        })
        .count();
    matched as f64 / keywords.len() as f64
}

/// Normalize Hamming distance to similarity in `[0, 1]`.
pub fn hamming_similarity(distance: u32, dimensions: usize) -> f64 {
    if dimensions == 0 {
        return 0.0;
    }
    1.0 - (distance as f64 / dimensions as f64)
}

/// Package / module key from a qualified name or parent directory.
pub fn entry_package_key(entry: &SemanticEntry) -> Option<String> {
    if let Some(qn) = &entry.qualified_name {
        let parts: Vec<&str> = qn
            .split(|c: char| c == '.' || c == ':' || c == '/')
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() >= 2 {
            let keep = parts.len().saturating_sub(2).max(1);
            return Some(parts[..keep].join(".").to_ascii_lowercase());
        }
    }
    entry.file_path.as_ref().and_then(|path| {
        std::path::Path::new(path)
            .parent()
            .and_then(|dir| dir.file_name())
            .map(|name| name.to_string_lossy().to_ascii_lowercase())
    })
}

fn callee_token_set(backend: &MemoryBackend, node_id: Uuid) -> HashSet<String> {
    let Ok(edges) = backend.get_outgoing_edges(node_id) else {
        return HashSet::new();
    };
    let mut tokens = HashSet::new();
    for edge in edges {
        if edge.edge_type != EdgeType::Calls {
            continue;
        }
        if let Ok(Some(node)) = backend.get_node(edge.to) {
            tokenize_string_into(&node.name, &mut tokens);
            if let Some(qn) = &node.qualified_name {
                tokenize_string_into(qn, &mut tokens);
            }
        }
    }
    tokens
}

fn jaccard(left: &HashSet<String>, right: &[String]) -> f64 {
    if right.is_empty() {
        return 0.0;
    }
    let matched = right.iter().filter(|token| left.contains(*token)).count();
    matched as f64 / right.len() as f64
}

/// Blend semantic and structural signals for Hamming candidates.
pub fn fuse_candidates(
    candidates: &mut [FusionCandidate],
    keywords: &[String],
    dimensions: usize,
    analysis: Option<&AnalysisResults>,
    backend: Option<&MemoryBackend>,
    config: &SemanticFusionConfig,
) {
    let max_pagerank = analysis
        .and_then(|results| results.centrality.as_ref())
        .map(|table| {
            table
                .pagerank
                .iter()
                .copied()
                .fold(0.0f32, f32::max)
                .max(f32::EPSILON)
        })
        .unwrap_or(1.0) as f64;

    let mut best_name = -1.0f64;
    let mut anchor_community: Option<usize> = None;
    let mut anchor_package: Option<String> = None;
    for candidate in candidates.iter() {
        let name_score = name_overlap_score(&candidate.entry, keywords);
        if name_score > best_name {
            best_name = name_score;
            anchor_community =
                analysis.and_then(|results| results.get_community(candidate.entry.node_id));
            anchor_package = entry_package_key(&candidate.entry);
        }
    }

    let mut callee_cache: HashMap<Uuid, HashSet<String>> = HashMap::new();

    for candidate in candidates.iter_mut() {
        let node_bloom = analysis
            .and_then(|results| results.get_compact_id(candidate.entry.node_id))
            .and_then(|compact_id| analysis?.structural_sketch.as_ref()?.bloom(compact_id));

        if config.keyword_and
            && !keyword_and_matches(&candidate.entry, keywords, node_bloom.as_ref())
        {
            candidate.fused_score = f64::NEG_INFINITY;
            continue;
        }

        let semantic = hamming_similarity(candidate.hamming_distance, dimensions);

        let blast_score = analysis
            .and_then(|results| results.get_blast_radius(candidate.entry.node_id))
            .map(|metrics| (metrics.score as f64 / 100.0).clamp(0.0, 1.0))
            .unwrap_or(0.0);

        let pagerank_score = analysis
            .and_then(|results| results.get_centrality(candidate.entry.node_id))
            .map(|metrics| (metrics.pagerank as f64 / max_pagerank).clamp(0.0, 1.0))
            .unwrap_or(0.0);

        let name_score = name_overlap_score(&candidate.entry, keywords);
        let sketch_score = node_bloom
            .as_ref()
            .map(|bloom| keyword_overlap_score(keywords, bloom))
            .unwrap_or(0.0);

        let community_score = match (anchor_community, analysis) {
            (Some(anchor), Some(results)) => results
                .get_community(candidate.entry.node_id)
                .map(|cid| if cid == anchor { 1.0 } else { 0.0 })
                .unwrap_or(0.0),
            _ => 0.0,
        };
        let package_score = match (&anchor_package, entry_package_key(&candidate.entry)) {
            (Some(anchor), Some(key)) if &key == anchor => 1.0,
            _ => 0.0,
        };
        let callee_score = backend
            .map(|graph| {
                let tokens = callee_cache
                    .entry(candidate.entry.node_id)
                    .or_insert_with(|| callee_token_set(graph, candidate.entry.node_id));
                jaccard(tokens, keywords)
            })
            .unwrap_or(0.0);

        candidate.fused_score = config.w_semantic * semantic
            + config.w_blast * blast_score
            + config.w_centrality * pagerank_score
            + config.w_name * name_score
            + config.w_sketch * sketch_score
            + config.w_community * community_score
            + config.w_package * package_score
            + config.w_callees * callee_score;
    }

    candidates.sort_by(|left, right| {
        right
            .fused_score
            .partial_cmp(&left.fused_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.hamming_distance.cmp(&right.hamming_distance))
            .then_with(|| left.row.cmp(&right.row))
    });
}

/// Search with optional two-stage Hamming + late fusion ranking.
pub fn query_index_with_fusion(
    index: &SemanticIndex,
    query: &str,
    limit: usize,
    reload: &OnnxReloadOptions,
    fusion: &SemanticFusionConfig,
    analysis: Option<&AnalysisResults>,
    backend: Option<&MemoryBackend>,
    _repo_root: Option<&Path>,
) -> Result<Vec<SemanticHit>> {
    let embedder = embedder_for_index(index, reload)?;
    let query_bits = embedder.embed_binary(query)?;
    let keywords = query_keywords(query);

    let pool = if fusion.enabled {
        fusion.candidate_pool.max(limit)
    } else {
        limit
    };

    let raw = hamming_top_k(index, &query_bits, pool);
    let mut candidates: Vec<FusionCandidate> = raw
        .into_iter()
        .filter_map(|(row, distance)| {
            let entry = index.entries.get(row)?.clone();
            Some(FusionCandidate {
                row,
                hamming_distance: distance,
                fused_score: 0.0,
                entry,
            })
        })
        .collect();

    if fusion.enabled && !candidates.is_empty() {
        fuse_candidates(
            &mut candidates,
            &keywords,
            index.dimensions,
            analysis,
            backend,
            fusion,
        );
    } else {
        candidates.sort_by(|left, right| {
            left.hamming_distance
                .cmp(&right.hamming_distance)
                .then_with(|| left.row.cmp(&right.row))
        });
    }

    Ok(candidates
        .into_iter()
        .filter(|candidate| candidate.fused_score.is_finite())
        .take(limit)
        .map(|candidate| SemanticHit {
            row: candidate.row,
            distance: candidate.hamming_distance,
            entry: candidate.entry,
            fused_score: if fusion.enabled {
                Some(candidate.fused_score)
            } else {
                None
            },
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::AnalysisResults;
    use uuid::Uuid;

    fn sample_entry(name: &str, qname: &str) -> SemanticEntry {
        SemanticEntry {
            node_id: Uuid::new_v4(),
            name: name.into(),
            qualified_name: Some(qname.into()),
            file_path: None,
            code_hash: None,
            node_type: None,
            kind: None,
        }
    }

    #[test]
    fn keyword_and_requires_all_terms() {
        let entry = sample_entry("processInvoice", "InvoiceService.processInvoice");
        assert!(keyword_and_matches(
            &entry,
            &["invoice".into(), "process".into()],
            None
        ));
        assert!(!keyword_and_matches(
            &entry,
            &["invoice".into(), "payment".into()],
            None
        ));
    }

    #[test]
    fn fusion_prefers_name_overlap_when_hamming_tied() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let mut candidates = vec![
            FusionCandidate {
                row: 0,
                hamming_distance: 80,
                fused_score: 0.0,
                entry: SemanticEntry {
                    node_id: id_a,
                    name: "cryptic".into(),
                    qualified_name: Some("svc.cryptic".into()),
                    file_path: None,
                    code_hash: None,
                    node_type: None,
                    kind: None,
                },
            },
            FusionCandidate {
                row: 1,
                hamming_distance: 80,
                fused_score: 0.0,
                entry: SemanticEntry {
                    node_id: id_b,
                    name: "processInvoice".into(),
                    qualified_name: Some("InvoiceService.processInvoice".into()),
                    file_path: None,
                    code_hash: None,
                    node_type: None,
                    kind: None,
                },
            },
        ];

        let config = SemanticFusionConfig {
            enabled: true,
            w_semantic: 0.0,
            w_blast: 0.0,
            w_centrality: 0.0,
            w_name: 1.0,
            w_sketch: 0.0,
            ..Default::default()
        };
        fuse_candidates(
            &mut candidates,
            &query_keywords("invoice process"),
            256,
            None,
            None,
            &config,
        );
        assert_eq!(candidates[0].entry.node_id, id_b);
    }

    #[test]
    fn fusion_uses_blast_and_centrality_from_analysis() {
        let uuid = Uuid::new_v4();
        let mut results = AnalysisResults::new(vec![uuid]);
        {
            let blast = results.init_blast_radius();
            blast.scores[0] = 90.0;
            let centrality = results.init_centrality();
            centrality.pagerank[0] = 1.0;
        }

        let mut candidates = vec![FusionCandidate {
            row: 0,
            hamming_distance: 50,
            fused_score: 0.0,
            entry: SemanticEntry {
                node_id: uuid,
                name: "anchor".into(),
                qualified_name: None,
                file_path: None,
                code_hash: None,
                node_type: None,
                kind: None,
            },
        }];

        let config = SemanticFusionConfig {
            enabled: true,
            w_semantic: 0.0,
            w_blast: 0.5,
            w_centrality: 0.5,
            w_name: 0.0,
            w_sketch: 0.0,
            ..Default::default()
        };
        fuse_candidates(&mut candidates, &[], 256, Some(&results), None, &config);
        assert!(candidates[0].fused_score > 0.8);
    }

    #[test]
    fn query_keywords_skip_short_tokens() {
        let keywords = query_keywords("a do invoice");
        assert!(keywords.iter().any(|token| token == "invoice"));
        assert!(!keywords.iter().any(|token| token == "a" || token == "do"));
        assert!(keywords.iter().all(|token| token.len() >= 3));
    }

    #[test]
    fn package_key_from_java_qn() {
        let entry = sample_entry("checkout", "com.example.cart.CartService.checkout");
        assert_eq!(
            entry_package_key(&entry).as_deref(),
            Some("com.example.cart")
        );
    }

    #[test]
    fn fusion_boosts_same_package_as_name_anchor() {
        let mut candidates = vec![
            FusionCandidate {
                row: 0,
                hamming_distance: 80,
                fused_score: 0.0,
                entry: sample_entry("checkout", "com.example.cart.CartService.checkout"),
            },
            FusionCandidate {
                row: 1,
                hamming_distance: 80,
                fused_score: 0.0,
                entry: sample_entry("helper", "com.other.util.Helper.helper"),
            },
        ];
        let config = SemanticFusionConfig {
            enabled: true,
            w_semantic: 0.0,
            w_blast: 0.0,
            w_centrality: 0.0,
            w_name: 0.0,
            w_sketch: 0.0,
            w_community: 0.0,
            w_package: 1.0,
            w_callees: 0.0,
            ..Default::default()
        };
        fuse_candidates(
            &mut candidates,
            &query_keywords("checkout cart"),
            256,
            None,
            None,
            &config,
        );
        assert_eq!(candidates[0].entry.name, "checkout");
        assert!(candidates[0].fused_score > candidates[1].fused_score);
    }
}
