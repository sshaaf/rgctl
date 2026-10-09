//! Exact (Type-1) clone detection via `code_hash` grouping.
//!
//! Query-time / sidecar only — does **not** write topology edges into
//! `graph.snapshot.bin`. See `docs/design/clone-detection-design.md`.

use rgctl_graph::paths::artifact_path;
use rgctl_graph::schema::{Node, NodeType};
use rgctl_graph::SnapshotNodeStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// JSON schema version for `rgctl clones` / `.rgctl/clones.json`.
pub const CLONE_REPORT_SCHEMA_VERSION: u32 = 1;

/// Sidecar filename under `.rgctl/`.
pub const CLONES_SIDECAR_FILE: &str = "clones.json";

/// Default minimum LOC for a function to participate in exact groups.
pub const DEFAULT_MIN_LOC: usize = 5;

/// Clone detection mode string for exact Type-1 groups.
pub const MODE_EXACT: &str = "exact";

/// Reserved mode string for bloom/sketch candidates (M2; not implemented).
pub const MODE_BLOOM: &str = "bloom";
/// Reserved mode string for embedding neighbor candidates (M2; not implemented).
pub const MODE_SEMANTIC: &str = "semantic";
/// Reserved mode string for PDG/CFG confirmation (M3; not implemented).
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
    #[error("unsupported clone mode '{0}' (MVP supports '{MODE_EXACT}' only)")]
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
    /// Detection mode (`exact`, future: `bloom` / `semantic` / `structural`).
    pub mode: String,
    /// Exact body hash when `mode == exact`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Member count (≥ 2 after filters).
    pub size: usize,
    /// Group members.
    pub members: Vec<CloneMember>,
    /// Optional confidence in \[0, 1\] for future non-exact modes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Optional similarity score for future candidate modes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

/// Versioned clone report (CLI stdout and `.rgctl/clones.json`).
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
    /// Number of groups.
    pub group_count: usize,
    /// Clone groups (sorted by size desc, then hash).
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

/// Path to `.rgctl/clones.json` for a session root.
pub fn clones_sidecar_path(repo_root: &Path) -> PathBuf {
    artifact_path(repo_root, CLONES_SIDECAR_FILE)
}

/// Load sidecar if present and `graph_digest` matches; otherwise `Ok(None)`.
pub fn load_sidecar_if_fresh(repo_root: &Path, graph_digest: &str) -> Result<Option<CloneReport>> {
    let path = clones_sidecar_path(repo_root);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    let report: CloneReport = serde_json::from_slice(&bytes)?;
    if report.graph_digest != graph_digest {
        return Ok(None);
    }
    Ok(Some(report))
}

/// Write clone report sidecar (creates `.rgctl/` as needed).
pub fn save_sidecar(repo_root: &Path, report: &CloneReport) -> Result<PathBuf> {
    let path = clones_sidecar_path(repo_root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(report)?;
    std::fs::write(&path, json)?;
    Ok(path)
}

/// Collect Function nodes with a non-empty `code_hash` from a snapshot store.
pub fn iter_hashed_functions(store: &SnapshotNodeStore) -> Result<Vec<Node>> {
    let Some(col) = store.columnar() else {
        return Err(CloneError::Graph(rgctl_error::Error::Other(
            "columnar snapshot required for clone detection (run `rgctl discover`)".into(),
        )));
    };
    let indexes = col.indexes_shared()?;
    let Some(ids) = indexes.1.get(&NodeType::Function) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for id in ids {
        let Some(node) = store.get_node(*id)? else {
            continue;
        };
        match node.code_hash.as_deref() {
            Some(h) if !h.is_empty() => out.push(node),
            _ => {}
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
    // Sidecar cache only for full-repo (no seed) with matching filters is complex;
    // cache only when no seed and default-ish filters — always rebuild for seed scope.
    if opts.seed_id.is_none() {
        if let Some(cached) = load_sidecar_if_fresh(repo_root, &digest)? {
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

/// Parse mode string; MVP accepts only `exact`.
pub fn parse_mode(mode: &str) -> Result<&'static str> {
    match mode.trim().to_ascii_lowercase().as_str() {
        "exact" => Ok(MODE_EXACT),
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
            group_count: 0,
            groups: vec![],
            seed: None,
        };
        save_sidecar(dir.path(), &report).unwrap();
        assert!(load_sidecar_if_fresh(dir.path(), "new").unwrap().is_none());
        assert!(load_sidecar_if_fresh(dir.path(), "old").unwrap().is_some());
    }

    #[test]
    fn parse_mode_exact_only() {
        assert_eq!(parse_mode("exact").unwrap(), MODE_EXACT);
        assert!(parse_mode("bloom").is_err());
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
