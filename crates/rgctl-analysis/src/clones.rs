//! Clone detection: exact Type-1 (`code_hash`) and bloom candidates (`token_bloom`).
//!
//! Query-time / sidecar only — does **not** write topology edges into
//! `graph.snapshot.bin`. See `docs/design/clone-detection-design.md`.

use rgctl_graph::paths::artifact_path;
use rgctl_graph::schema::{Node, NodeType};
use rgctl_graph::structural_sketch::{TokenBloom, bloom_jaccard, bloom_popcount};
use rgctl_graph::SnapshotNodeStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

/// JSON schema version for `rgctl clones` / `.rgctl/clones*.json`.
pub const CLONE_REPORT_SCHEMA_VERSION: u32 = 1;

/// Sidecar filename under `.rgctl/` for exact mode.
pub const CLONES_SIDECAR_FILE: &str = "clones.json";

/// Default minimum LOC for a function to participate in clone groups.
pub const DEFAULT_MIN_LOC: usize = 5;

/// Default minimum Jaccard for bloom candidates.
pub const DEFAULT_BLOOM_THRESHOLD: f64 = 0.85;

/// Skip pairwise expansion when an LSH band bucket exceeds this size.
pub const DEFAULT_BLOOM_MAX_BUCKET: usize = 512;

/// Clone detection mode string for exact Type-1 groups.
pub const MODE_EXACT: &str = "exact";

/// Mode string for token-bloom Jaccard candidates (weak near-duplicates).
pub const MODE_BLOOM: &str = "bloom";
/// Reserved mode string for embedding neighbor candidates (not implemented).
pub const MODE_SEMANTIC: &str = "semantic";
/// Reserved mode string for PDG/CFG confirmation (not implemented).
pub const MODE_STRUCTURAL: &str = "structural";

/// Errors from clone detection / sidecar I/O.
#[derive(Debug, Error)]
pub enum CloneError {
    /// Snapshot missing or unreadable.
    #[error("{0}")]
    Graph(#[from] rgctl_error::Error),
    /// JSON serialize/deserialize failure.
    #[error("clone report JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Filesystem I/O.
    #[error("clone sidecar I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Unsupported mode for this build.
    #[error(
        "unsupported clone mode '{0}' (supported: '{MODE_EXACT}', '{MODE_BLOOM}'; reserved: '{MODE_SEMANTIC}', '{MODE_STRUCTURAL}')"
    )]
    UnsupportedMode(String),
}

/// Result alias for clone APIs.
pub type Result<T> = std::result::Result<T, CloneError>;

/// Filters applied when building exact clone groups.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneFilters {
    /// Minimum inclusive LOC (`end_line - start_line + 1`); default [`DEFAULT_MIN_LOC`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_loc: Option<usize>,
    /// Path substring / simple glob excludes (e.g. `test`, `**/generated/**`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    /// Optional language filter (`java`, `c`, …) matched against node property or extension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

impl CloneFilters {
    /// Filters with default `min_loc`.
    pub fn with_defaults() -> Self {
        Self {
            min_loc: Some(DEFAULT_MIN_LOC),
            exclude: Vec::new(),
            language: None,
        }
    }

    fn effective_min_loc(&self) -> usize {
        self.min_loc.unwrap_or(DEFAULT_MIN_LOC)
    }
}

/// One function member of a clone group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloneMember {
    /// Node UUID.
    pub id: String,
    /// Symbol name.
    pub name: String,
    /// Source file path when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Start line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_line: Option<usize>,
    /// End line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<usize>,
    /// Derived LOC when span is known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loc: Option<usize>,
}

impl CloneMember {
    fn from_node(node: &Node) -> Self {
        let loc = match (node.start_line, node.end_line) {
            (Some(s), Some(e)) if e >= s => Some(e - s + 1),
            _ => node
                .get_property("loc")
                .and_then(|v| v.parse::<usize>().ok()),
        };
        Self {
            id: node.id.to_string(),
            name: node.name.to_string(),
            file: node.file_path.as_ref().map(|s| s.to_string()),
            start_line: node.start_line,
            end_line: node.end_line,
            loc,
        }
    }
}

/// A group of functions sharing the same similarity key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloneGroup {
    /// Detection mode (`exact`, `bloom`, …).
    pub mode: String,
    /// Exact body hash when `mode == exact`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Member count (≥ 2 after filters).
    pub size: usize,
    /// Group members.
    pub members: Vec<CloneMember>,
    /// Confidence in \[0, 1\] (`1.0` for exact; Jaccard for bloom).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Similarity score (bloom: min pairwise Jaccard in the group).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// Versioned clone report (CLI stdout and `.rgctl/clones*.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloneReport {
    /// Schema version.
    pub schema_version: u32,
    /// Requested / computed mode.
    pub mode: String,
    /// Graph content digest used for sidecar invalidation.
    pub graph_digest: String,
    /// Filters that were applied.
    pub filters: CloneFilters,
    /// Bloom min Jaccard (present for `mode == bloom`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    /// True when groups are similarity **candidates** (not Type-1 exact).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub candidates: bool,
    /// Number of groups.
    pub group_count: usize,
    /// Clone groups (sorted by size/score desc).
    pub groups: Vec<CloneGroup>,
    /// Seed member when report is symbol-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<CloneMember>,
}

/// Options for building an exact clone report.
#[derive(Debug, Clone)]
pub struct ExactCloneOptions {
    /// Filters.
    pub filters: CloneFilters,
    /// When set, keep only the group containing this node id (plus report seed).
    pub seed_id: Option<Uuid>,
}

impl Default for ExactCloneOptions {
    fn default() -> Self {
        Self {
            filters: CloneFilters::with_defaults(),
            seed_id: None,
        }
    }
}

/// Options for bloom candidate clone detection.
#[derive(Debug, Clone)]
pub struct BloomCloneOptions {
    /// Filters (min LOC / path / language).
    pub filters: CloneFilters,
    /// When set, report only candidates of this function.
    pub seed_id: Option<Uuid>,
    /// Minimum Jaccard in \[0, 1\] (default [`DEFAULT_BLOOM_THRESHOLD`]).
    pub threshold: f64,
    /// Max functions per LSH band bucket before skipping pairwise (default [`DEFAULT_BLOOM_MAX_BUCKET`]).
    pub max_bucket: usize,
}

impl Default for BloomCloneOptions {
    fn default() -> Self {
        Self {
            filters: CloneFilters::with_defaults(),
            seed_id: None,
            threshold: DEFAULT_BLOOM_THRESHOLD,
            max_bucket: DEFAULT_BLOOM_MAX_BUCKET,
        }
    }
}

/// Path to `.rgctl/clones.json` (exact mode) for a session root.
pub fn clones_sidecar_path(repo_root: &Path) -> PathBuf {
    clones_sidecar_path_for_mode(repo_root, MODE_EXACT)
}

/// Sidecar path for a clone mode (`clones.json` or `clones.<mode>.json`).
pub fn clones_sidecar_path_for_mode(repo_root: &Path, mode: &str) -> PathBuf {
    if mode == MODE_EXACT {
        artifact_path(repo_root, CLONES_SIDECAR_FILE)
    } else {
        artifact_path(repo_root, format!("clones.{mode}.json"))
    }
}

/// Load sidecar if present and `graph_digest` matches; otherwise `Ok(None)`.
pub fn load_sidecar_if_fresh(
    repo_root: &Path,
    graph_digest: &str,
    mode: &str,
) -> Result<Option<CloneReport>> {
    let path = clones_sidecar_path_for_mode(repo_root, mode);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    let report: CloneReport = serde_json::from_slice(&bytes)?;
    if report.graph_digest != graph_digest || report.mode != mode {
        return Ok(None);
    }
    Ok(Some(report))
}

/// Write clone report sidecar (creates `.rgctl/` as needed).
pub fn save_sidecar(repo_root: &Path, report: &CloneReport) -> Result<PathBuf> {
    let path = clones_sidecar_path_for_mode(repo_root, &report.mode);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(report)?;
    std::fs::write(&path, json)?;
    Ok(path)
}

fn open_function_ids(store: &SnapshotNodeStore) -> Result<Vec<Uuid>> {
    let Some(col) = store.columnar() else {
        return Err(CloneError::Graph(rgctl_error::Error::Other(
            "columnar snapshot required for clone detection (run `rgctl discover`)".into(),
        )));
    };
    let indexes = col.indexes_shared()?;
    Ok(indexes
        .1
        .get(&NodeType::Function)
        .cloned()
        .unwrap_or_default())
}

/// Collect Function nodes with a non-empty `code_hash` from a snapshot store.
pub fn iter_hashed_functions(store: &SnapshotNodeStore) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    for id in open_function_ids(store)? {
        let Some(node) = store.get_node(id)? else {
            continue;
        };
        match node.code_hash.as_deref() {
            Some(h) if !h.is_empty() => out.push(node),
            _ => {}
        }
    }
    Ok(out)
}

/// Collect Function nodes with a non-empty `token_bloom` from a snapshot store.
pub fn iter_bloom_functions(store: &SnapshotNodeStore) -> Result<Vec<Node>> {
    let mut out = Vec::new();
    for id in open_function_ids(store)? {
        let Some(node) = store.get_node(id)? else {
            continue;
        };
        if let Some(bloom) = node.token_bloom.as_ref() {
            if bloom_popcount(bloom) > 0 {
                out.push(node);
            }
        }
    }
    Ok(out)
}

fn member_passes(node: &Node, filters: &CloneFilters) -> bool {
    let loc = match (node.start_line, node.end_line) {
        (Some(s), Some(e)) if e >= s => e - s + 1,
        _ => 0,
    };
    if loc < filters.effective_min_loc() {
        return false;
    }
    if let Some(ref path) = node.file_path {
        if path_excluded(path.as_ref(), &filters.exclude) {
            return false;
        }
    }
    if let Some(ref want) = filters.language {
        if !language_matches(node, want) {
            return false;
        }
    }
    true
}

fn language_matches(node: &Node, want: &str) -> bool {
    let want = want.to_ascii_lowercase();
    if let Some(lang) = node.get_property("language") {
        if lang.eq_ignore_ascii_case(&want) {
            return true;
        }
    }
    let Some(path) = node.file_path.as_deref() else {
        return false;
    };
    let path: &str = path.as_ref();
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    language_from_ext(ext) == want
}

fn language_from_ext(ext: &str) -> String {
    match ext.to_ascii_lowercase().as_str() {
        "java" => "java".into(),
        "c" | "h" => "c".into(),
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => "cpp".into(),
        "rs" => "rust".into(),
        "py" => "python".into(),
        "go" => "go".into(),
        "js" | "mjs" | "cjs" => "javascript".into(),
        "ts" | "tsx" => "typescript".into(),
        "rb" => "ruby".into(),
        "php" => "php".into(),
        "cs" => "csharp".into(),
        "kt" | "kts" => "kotlin".into(),
        "groovy" => "groovy".into(),
        other => other.to_string(),
    }
}

/// Path exclude: bare tokens match a **path component** (so `test` does not
/// match `rgctl-tests`); patterns with `/` match as a path substring.
pub fn path_excluded(path: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let norm = path.replace('\\', "/").to_ascii_lowercase();
    let components: Vec<&str> = norm.split('/').filter(|c| !c.is_empty()).collect();
    for raw in patterns {
        let pat = raw.replace('\\', "/").to_ascii_lowercase();
        let needle = pat
            .trim_matches('*')
            .trim_matches('/')
            .replace("**", "")
            .replace('*', "");
        let needle = needle.trim_matches('/').to_string();
        if needle.is_empty() {
            continue;
        }
        if needle.contains('/') {
            let hay = format!("/{norm}/");
            if hay.contains(&format!("/{needle}/")) {
                return true;
            }
        } else if components.iter().any(|c| *c == needle) {
            return true;
        }
    }
    false
}

/// Build exact Type-1 clone groups from already-loaded function nodes.
pub fn group_exact_from_nodes(nodes: &[Node], opts: &ExactCloneOptions) -> Vec<CloneGroup> {
    let mut buckets: HashMap<&str, Vec<&Node>> = HashMap::new();
    for node in nodes {
        if node.node_type != NodeType::Function {
            continue;
        }
        let Some(hash) = node.code_hash.as_deref().filter(|h| !h.is_empty()) else {
            continue;
        };
        if !member_passes(node, &opts.filters) {
            continue;
        }
        buckets.entry(hash).or_default().push(node);
    }

    let mut groups = Vec::new();
    for (hash, members) in buckets {
        if members.len() < 2 {
            continue;
        }
        if let Some(seed) = opts.seed_id {
            if !members.iter().any(|n| n.id == seed) {
                continue;
            }
        }
        let mut clone_members: Vec<CloneMember> =
            members.iter().map(|n| CloneMember::from_node(n)).collect();
        clone_members.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.start_line.cmp(&b.start_line))
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        groups.push(CloneGroup {
            mode: MODE_EXACT.to_string(),
            hash: Some(hash.to_string()),
            size: clone_members.len(),
            members: clone_members,
            confidence: Some(1.0),
            score: None,
        });
    }
    groups.sort_by(|a, b| {
        b.size
            .cmp(&a.size)
            .then(a.hash.cmp(&b.hash))
    });
    groups
}

/// Scan the snapshot and build an exact clone report (no sidecar write).
pub fn build_exact_report(store: &SnapshotNodeStore, opts: ExactCloneOptions) -> Result<CloneReport> {
    let digest = store.content_digest()?.to_string();
    let nodes = iter_hashed_functions(store)?;
    let groups = group_exact_from_nodes(&nodes, &opts);
    let seed = opts.seed_id.and_then(|id| {
        nodes
            .iter()
            .find(|n| n.id == id)
            .map(CloneMember::from_node)
    });
    Ok(CloneReport {
        schema_version: CLONE_REPORT_SCHEMA_VERSION,
        mode: MODE_EXACT.to_string(),
        graph_digest: digest,
        filters: opts.filters,
        threshold: None,
        candidates: false,
        group_count: groups.len(),
        groups,
        seed,
    })
}

/// Build exact report, optionally reusing a fresh sidecar; write sidecar when rebuilt.
pub fn exact_clones_with_cache(
    store: &SnapshotNodeStore,
    repo_root: &Path,
    opts: ExactCloneOptions,
    write_sidecar: bool,
) -> Result<CloneReport> {
    let digest = store.content_digest()?.to_string();
    if opts.seed_id.is_none() {
        if let Some(cached) = load_sidecar_if_fresh(repo_root, &digest, MODE_EXACT)? {
            if cached.mode == MODE_EXACT && cached.filters == opts.filters {
                return Ok(cached);
            }
        }
    }
    let report = build_exact_report(store, opts)?;
    if write_sidecar && report.seed.is_none() {
        save_sidecar(repo_root, &report)?;
    }
    Ok(report)
}

fn bloom_of(node: &Node) -> Option<&TokenBloom> {
    node.token_bloom.as_ref().filter(|b| bloom_popcount(b) > 0)
}

/// Build bloom candidate groups from nodes (unit-testable; no snapshot I/O).
///
/// Uses 64-bit LSH bands (each `token_bloom` word) to generate candidate pairs,
/// then keeps pairs with Jaccard ≥ `threshold`. Connected components become groups;
/// group `score` / `confidence` = minimum edge Jaccard in the component.
pub fn group_bloom_from_nodes(nodes: &[Node], opts: &BloomCloneOptions) -> Vec<CloneGroup> {
    let threshold = opts.threshold.clamp(0.0, 1.0);
    let mut indexed: Vec<&Node> = Vec::new();
    for node in nodes {
        if node.node_type != NodeType::Function {
            continue;
        }
        if bloom_of(node).is_none() {
            continue;
        }
        if !member_passes(node, &opts.filters) {
            continue;
        }
        indexed.push(node);
    }
    if indexed.len() < 2 {
        return Vec::new();
    }

    // Seed-scoped: O(n) vs seed only (no full-repo LSH).
    if let Some(seed_id) = opts.seed_id {
        let Some(seed_pos) = indexed.iter().position(|n| n.id == seed_id) else {
            return Vec::new();
        };
        let seed_bloom = bloom_of(indexed[seed_pos]).expect("filtered");
        let mut members = vec![CloneMember::from_node(indexed[seed_pos])];
        let mut min_score = 1.0_f64;
        for (i, node) in indexed.iter().enumerate() {
            if i == seed_pos {
                continue;
            }
            let Some(b) = bloom_of(node) else {
                continue;
            };
            let j = bloom_jaccard(seed_bloom, b);
            if j + f64::EPSILON >= threshold {
                min_score = min_score.min(j);
                members.push(CloneMember::from_node(node));
            }
        }
        if members.len() < 2 {
            return Vec::new();
        }
        members.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.start_line.cmp(&b.start_line))
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        return vec![CloneGroup {
            mode: MODE_BLOOM.to_string(),
            hash: None,
            size: members.len(),
            members,
            confidence: Some(min_score),
            score: Some(min_score),
        }];
    }

    // LSH: each non-zero u64 word is a band key.
    let mut bands: HashMap<(u8, u64), Vec<usize>> = HashMap::new();
    for (idx, node) in indexed.iter().enumerate() {
        let bloom = bloom_of(node).expect("filtered");
        for (band, word) in bloom.iter().enumerate() {
            if *word != 0 {
                bands
                    .entry((band as u8, *word))
                    .or_default()
                    .push(idx);
            }
        }
    }

    let n = indexed.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut min_edge: HashMap<(usize, usize), f64> = HashMap::new();

    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut x = x;
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    fn unite(parent: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent[rb] = ra;
        }
    }

    for ((_band, _word), bucket) in bands {
        if bucket.len() < 2 {
            continue;
        }
        if bucket.len() > opts.max_bucket {
            warn!(
                bucket = bucket.len(),
                max = opts.max_bucket,
                "bloom LSH bucket oversized; skipping pairwise expansion"
            );
            continue;
        }
        for i in 0..bucket.len() {
            for j in (i + 1)..bucket.len() {
                let a = bucket[i];
                let b = bucket[j];
                let ba = bloom_of(indexed[a]).expect("filtered");
                let bb = bloom_of(indexed[b]).expect("filtered");
                let jacc = bloom_jaccard(ba, bb);
                if jacc + f64::EPSILON >= threshold {
                    unite(&mut parent, a, b);
                    let key = if a < b { (a, b) } else { (b, a) };
                    min_edge
                        .entry(key)
                        .and_modify(|e| *e = e.min(jacc))
                        .or_insert(jacc);
                }
            }
        }
    }

    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        comps.entry(root).or_default().push(i);
    }

    let mut groups = Vec::new();
    for members_idx in comps.into_values() {
        if members_idx.len() < 2 {
            continue;
        }
        // Min edge Jaccard among pairs that were united (conservative).
        let mut score = 1.0_f64;
        let mut saw_edge = false;
        for i in 0..members_idx.len() {
            for j in (i + 1)..members_idx.len() {
                let a = members_idx[i];
                let b = members_idx[j];
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some(e) = min_edge.get(&key) {
                    score = score.min(*e);
                    saw_edge = true;
                }
            }
        }
        if !saw_edge {
            // Component formed but no recorded edge (shouldn't happen); recompute.
            for i in 0..members_idx.len() {
                for j in (i + 1)..members_idx.len() {
                    let ba = bloom_of(indexed[members_idx[i]]).expect("filtered");
                    let bb = bloom_of(indexed[members_idx[j]]).expect("filtered");
                    score = score.min(bloom_jaccard(ba, bb));
                }
            }
        }
        if score + f64::EPSILON < threshold {
            continue;
        }
        let mut members: Vec<CloneMember> = members_idx
            .iter()
            .map(|&i| CloneMember::from_node(indexed[i]))
            .collect();
        members.sort_by(|a, b| {
            a.file
                .cmp(&b.file)
                .then(a.start_line.cmp(&b.start_line))
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        groups.push(CloneGroup {
            mode: MODE_BLOOM.to_string(),
            hash: None,
            size: members.len(),
            members,
            confidence: Some(score),
            score: Some(score),
        });
    }

    groups.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.size.cmp(&a.size))
            .then(a.members[0].id.cmp(&b.members[0].id))
    });
    groups
}

/// Scan the snapshot and build a bloom candidate report (no sidecar write).
pub fn build_bloom_report(store: &SnapshotNodeStore, opts: BloomCloneOptions) -> Result<CloneReport> {
    let digest = store.content_digest()?.to_string();
    let nodes = iter_bloom_functions(store)?;
    let groups = group_bloom_from_nodes(&nodes, &opts);
    let seed = opts.seed_id.and_then(|id| {
        nodes
            .iter()
            .find(|n| n.id == id)
            .map(CloneMember::from_node)
    });
    Ok(CloneReport {
        schema_version: CLONE_REPORT_SCHEMA_VERSION,
        mode: MODE_BLOOM.to_string(),
        graph_digest: digest,
        filters: opts.filters,
        threshold: Some(opts.threshold),
        candidates: true,
        group_count: groups.len(),
        groups,
        seed,
    })
}

/// Build bloom report with optional sidecar cache (`.rgctl/clones.bloom.json`).
pub fn bloom_clones_with_cache(
    store: &SnapshotNodeStore,
    repo_root: &Path,
    opts: BloomCloneOptions,
    write_sidecar: bool,
) -> Result<CloneReport> {
    let digest = store.content_digest()?.to_string();
    if opts.seed_id.is_none() {
        if let Some(cached) = load_sidecar_if_fresh(repo_root, &digest, MODE_BLOOM)? {
            if cached.mode == MODE_BLOOM
                && cached.filters == opts.filters
                && cached.threshold == Some(opts.threshold)
            {
                return Ok(cached);
            }
        }
    }
    let report = build_bloom_report(store, opts)?;
    if write_sidecar && report.seed.is_none() {
        save_sidecar(repo_root, &report)?;
    }
    Ok(report)
}

/// Parse mode string (`exact` | `bloom`).
pub fn parse_mode(mode: &str) -> Result<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "exact" => Ok(MODE_EXACT),
        "bloom" | "sketch" => Ok(MODE_BLOOM),
        other => Err(CloneError::UnsupportedMode(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgctl_graph::schema::Node;

    fn fn_node(name: &str, file: &str, hash: &str, start: usize, end: usize) -> Node {
        Node::new(NodeType::Function, name)
            .with_file_path(file)
            .with_location(start, end)
            .with_code_hash(hash)
    }

    #[test]
    fn groups_identical_hashes_size_ge_2() {
        let nodes = vec![
            fn_node("dupA", "a.java", "abc", 1, 20),
            fn_node("dupB", "b.java", "abc", 1, 20),
            fn_node("unique", "c.java", "zzz", 1, 20),
        ];
        let groups = group_exact_from_nodes(&nodes, &ExactCloneOptions::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].size, 2);
        assert_eq!(groups[0].hash.as_deref(), Some("abc"));
        assert_eq!(groups[0].mode, MODE_EXACT);
        assert_eq!(groups[0].confidence, Some(1.0));
    }

    #[test]
    fn skips_missing_hash_and_singletons() {
        let mut no_hash = Node::new(NodeType::Function, "bare").with_file_path("x.java");
        no_hash.start_line = Some(1);
        no_hash.end_line = Some(20);
        let nodes = vec![
            no_hash,
            fn_node("only", "y.java", "solo", 1, 20),
        ];
        let groups = group_exact_from_nodes(&nodes, &ExactCloneOptions::default());
        assert!(groups.is_empty());
    }

    #[test]
    fn min_loc_drops_group() {
        let nodes = vec![
            fn_node("a", "a.java", "h", 1, 2), // loc=2
            fn_node("b", "b.java", "h", 1, 2),
        ];
        let mut opts = ExactCloneOptions::default();
        opts.filters.min_loc = Some(5);
        assert!(group_exact_from_nodes(&nodes, &opts).is_empty());
    }

    #[test]
    fn path_exclude_filters_members() {
        let nodes = vec![
            fn_node("a", "src/Main.java", "h", 1, 20),
            fn_node("b", "src/test/MainTest.java", "h", 1, 20),
        ];
        let mut opts = ExactCloneOptions::default();
        opts.filters.exclude = vec!["test".into()];
        // only one member remains → no group
        assert!(group_exact_from_nodes(&nodes, &opts).is_empty());
    }

    #[test]
    fn seed_keeps_only_matching_group() {
        let a = fn_node("a", "a.java", "h1", 1, 20);
        let b = fn_node("b", "b.java", "h1", 1, 20);
        let c = fn_node("c", "c.java", "h2", 1, 20);
        let d = fn_node("d", "d.java", "h2", 1, 20);
        let seed = a.id;
        let nodes = vec![a, b, c, d];
        let opts = ExactCloneOptions {
            filters: CloneFilters::with_defaults(),
            seed_id: Some(seed),
        };
        let groups = group_exact_from_nodes(&nodes, &opts);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].hash.as_deref(), Some("h1"));
    }

    #[test]
    fn sidecar_stale_on_digest_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let report = CloneReport {
            schema_version: CLONE_REPORT_SCHEMA_VERSION,
            mode: MODE_EXACT.into(),
            graph_digest: "old".into(),
            filters: CloneFilters::with_defaults(),
            threshold: None,
            candidates: false,
            group_count: 0,
            groups: vec![],
            seed: None,
        };
        save_sidecar(dir.path(), &report).unwrap();
        assert!(load_sidecar_if_fresh(dir.path(), "new", MODE_EXACT)
            .unwrap()
            .is_none());
        assert!(load_sidecar_if_fresh(dir.path(), "old", MODE_EXACT)
            .unwrap()
            .is_some());
    }

    #[test]
    fn parse_mode_exact_and_bloom() {
        assert_eq!(parse_mode("exact").unwrap(), MODE_EXACT);
        assert_eq!(parse_mode("bloom").unwrap(), MODE_BLOOM);
        assert_eq!(parse_mode("sketch").unwrap(), MODE_BLOOM);
        assert!(parse_mode("semantic").is_err());
    }

    #[test]
    fn bloom_groups_high_jaccard() {
        use rgctl_graph::structural_sketch::build_token_bloom;
        let body = "alpha beta gamma delta epsilon zeta eta theta";
        let a = Node::new(NodeType::Function, "nearA")
            .with_file_path("a.java")
            .with_location(1, 20)
            .with_token_bloom(build_token_bloom("nearA", None, None, Some(body)));
        let b = Node::new(NodeType::Function, "nearB")
            .with_file_path("b.java")
            .with_location(1, 20)
            .with_token_bloom(build_token_bloom("nearB", None, None, Some(body)));
        let c = Node::new(NodeType::Function, "far")
            .with_file_path("c.java")
            .with_location(1, 20)
            .with_token_bloom(build_token_bloom(
                "far",
                None,
                None,
                Some("zzzz yyyy xxxx wwww vvvv uuuu"),
            ));
        let groups = group_bloom_from_nodes(&[a, b, c], &BloomCloneOptions::default());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].mode, MODE_BLOOM);
        assert_eq!(groups[0].size, 2);
        assert!(groups[0].score.unwrap() >= DEFAULT_BLOOM_THRESHOLD);
        let names: Vec<_> = groups[0].members.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"nearA") && names.contains(&"nearB"));
        assert!(!names.contains(&"far"));
    }

    #[test]
    fn simple_glob_exclude() {
        assert!(path_excluded("foo/generated/Bar.java", &["**/generated/**".into()]));
        assert!(!path_excluded("foo/src/Bar.java", &["**/generated/**".into()]));
        // Must not treat `rgctl-tests` as the component `test`
        assert!(!path_excluded(
            "/Users/x/rgctl-tests/clone-exact/src/CloneA.java",
            &["test".into()]
        ));
        assert!(path_excluded(
            "/Users/x/clone-exact/src/test/CloneTestDup.java",
            &["test".into()]
        ));
    }
}
