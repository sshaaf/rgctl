//! Git-scoped entity resolution for PR policy gates.

use crate::file_tracker::ChangeSet;
use rgctl_error::{Error, Result};
use rgctl_graph::SnapshotNodeStore;
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::normalize_path_str;
use rgctl_graph::schema::NodeType;
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_graph::stable_key::{
    StableNodeKey, node_row_ref, node_scope_path_at, stable_key_from_row,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Normalized repo-relative paths touched between two git refs.
#[derive(Debug, Clone, Default)]
pub struct ScopedPaths {
    files: HashSet<Arc<str>>,
    change_set: ChangeSet,
}

impl ScopedPaths {
    /// Build from `git diff --name-status -z base_ref head_ref`.
    pub fn from_git_refs(repo_root: &Path, base_ref: &str, head_ref: &str) -> Result<Self> {
        let change_set = git_diff_name_status(repo_root, base_ref, head_ref)?;
        Ok(Self::from_change_set(change_set))
    }

    /// Build from a parsed git or tracker change set.
    pub fn from_change_set(change_set: ChangeSet) -> Self {
        let files = change_set
            .scoped_paths()
            .into_iter()
            .map(|p| Arc::<str>::from(normalize_path_str(&p)))
            .collect();
        Self {
            files,
            change_set,
        }
    }

    /// Build from an explicit path list (normalized).
    pub fn from_paths(paths: Vec<String>) -> Self {
        let files = paths
            .iter()
            .map(|p| Arc::<str>::from(normalize_path_str(p)))
            .collect();
        Self {
            files,
            change_set: ChangeSet::default(),
        }
    }

    /// Git name-status breakdown for this scope.
    pub fn change_set(&self) -> &ChangeSet {
        &self.change_set
    }

    /// Paths that must be invalidated in a base snapshot before delta compact.
    pub fn invalidation_paths(&self) -> Vec<String> {
        if self.change_set.is_empty() {
            return self
                .files
                .iter()
                .map(|p| p.to_string())
                .collect();
        }
        self.change_set.invalidation_paths()
    }

    /// True when the path set is empty.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Number of scoped paths.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether a normalized file path is in scope.
    pub fn contains_path(&self, path: &str) -> bool {
        let normalized = normalize_path_str(path);
        self.files.contains(normalized.as_str())
            || self
                .files
                .iter()
                .any(|p| normalized.ends_with(p.as_ref()) || p.as_ref().ends_with(&normalized))
    }

    /// Interned path set.
    pub fn files(&self) -> &HashSet<Arc<str>> {
        &self.files
    }
}

/// Inclusive line range on the post-image (head) side of a diff hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

impl LineRange {
    fn overlaps(&self, other_start: u32, other_end: u32) -> bool {
        if self.start == 0 || other_start == 0 {
            return true;
        }
        self.start <= other_end && other_start <= self.end
    }
}

/// Parsed `+` side line ranges per file from a unified diff.
#[derive(Debug, Clone, Default)]
pub struct HunkIndex {
    ranges: std::collections::HashMap<Arc<str>, Vec<LineRange>>,
}

impl HunkIndex {
    /// Parse unified diff output (`git diff -U0 base head`).
    pub fn from_unified_diff(diff: &str) -> Self {
        let mut index = Self::default();
        let mut current_file: Option<Arc<str>> = None;

        for line in diff.lines() {
            if let Some(path) = line.strip_prefix("+++ b/") {
                current_file = Some(Arc::from(normalize_path_str(path)));
                continue;
            }
            if let Some(header) = line.strip_prefix("@@ ") {
                let Some(file) = current_file.clone() else {
                    continue;
                };
                if let Some(range) = parse_hunk_new_range(header) {
                    index
                        .ranges
                        .entry(file)
                        .or_default()
                        .push(range);
                }
            }
        }
        index
    }

    /// Line ranges for a normalized file path, if any.
    pub fn ranges_for(&self, path: &str) -> Option<&[LineRange]> {
        let normalized = normalize_path_str(path);
        self.ranges.get(normalized.as_str()).map(|v| v.as_slice())
    }
}

fn parse_hunk_new_range(header: &str) -> Option<LineRange> {
    // `@@ -10,5 +20,6 @@` → take `+20,6`
    let plus = header.split(' ').find(|part| part.starts_with('+'))?;
    let body = plus.trim_start_matches('+');
    let (start, count) = if let Some((s, c)) = body.split_once(',') {
        (s.parse::<u32>().ok()?, c.parse::<u32>().ok()?)
    } else {
        (body.parse::<u32>().ok()?, 1)
    };
    if start == 0 {
        return None;
    }
    let end = start.saturating_add(count.saturating_sub(1));
    Some(LineRange { start, end })
}

/// PR-scoped view over the head snapshot.
pub struct EntityScope<'a> {
    head: &'a SnapshotNodeStore,
    paths: &'a ScopedPaths,
    hunks: Option<&'a HunkIndex>,
}

impl<'a> EntityScope<'a> {
    /// Create a scope over head nodes constrained by git paths and optional hunks.
    pub fn new(head: &'a SnapshotNodeStore, paths: &'a ScopedPaths, hunks: Option<&'a HunkIndex>) -> Self {
        Self {
            head,
            paths,
            hunks,
        }
    }

    /// Stable keys for head entities overlapping the scoped diff.
    pub fn changed_entities(&self) -> Result<Vec<StableNodeKey>> {
        let col = self
            .head
            .columnar()
            .ok_or_else(|| Error::Other("entity scope requires columnar v2 snapshot".into()))?;
        let mut keys = Vec::new();

        for idx in 0..col.node_count() {
            let Some(path) = node_scope_path_at(col, idx)? else {
                continue;
            };
            if !self.paths.contains_path(path) {
                continue;
            }
            let row_ref = node_row_ref(col, idx)?;
            if let Some(hunks) = self.hunks {
                if let Some(ranges) = hunks.ranges_for(path) {
                    if row_ref.start_line > 0
                        && !ranges
                            .iter()
                            .any(|r| r.overlaps(row_ref.start_line, row_ref.end_line))
                    {
                        continue;
                    }
                }
            }
            keys.push(stable_key_from_row(col, idx)?);
        }
        Ok(keys)
    }
}

/// Options for resolving changed function symbols from git.
#[derive(Debug, Clone)]
pub struct SymbolScopeOptions {
    pub base_ref: Option<String>,
    pub head_ref: Option<String>,
    pub strict: bool,
}

/// Resolve function symbol names in the git diff between refs (or working tree vs HEAD).
pub fn changed_function_symbols(
    repo: &Path,
    backend: &MemoryBackend,
    options: &SymbolScopeOptions,
) -> Result<Vec<String>> {
    let paths = match (&options.base_ref, &options.head_ref) {
        (Some(base), Some(head)) => git_diff_name_only(repo, base, head)?,
        _ => git_diff_worktree_name_only(repo)?,
    };

    if paths.is_empty() {
        if options.strict {
            return Err(Error::Other(
                "strict diff scope: no changed files between refs".into(),
            ));
        }
        return Ok(all_function_symbols(backend));
    }

    let symbols = function_symbols_in_paths(backend, &paths);
    if symbols.is_empty() && options.strict {
        return Err(Error::Other(
            "strict diff scope: no functions matched changed paths".into(),
        ));
    }
    if symbols.is_empty() && !options.strict {
        return Ok(all_function_symbols(backend));
    }
    Ok(symbols)
}

/// Function symbol names whose file path overlaps any changed path.
pub fn function_symbols_in_paths(backend: &MemoryBackend, paths: &[String]) -> Vec<String> {
    let mut symbols = Vec::new();
    for node in backend.all_nodes().unwrap_or_default() {
        if node.node_type != NodeType::Function {
            continue;
        }
        if let Some(ref fp) = node.file_path {
            if paths.iter().any(|p| {
                let normalized = normalize_path_str(p);
                fp.ends_with(&normalized) || normalized.ends_with(fp.as_str())
            }) {
                symbols.push(node.name.to_string());
            }
        }
    }
    symbols
}

fn all_function_symbols(backend: &MemoryBackend) -> Vec<String> {
    backend
        .collect_nodes_by_type(NodeType::Function)
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.name.to_string())
        .collect()
}

/// `git diff --name-only base_ref head_ref` → normalized repo-relative paths.
pub fn git_diff_name_only(repo_root: &Path, base_ref: &str, head_ref: &str) -> Result<Vec<String>> {
    Ok(git_diff_name_status(repo_root, base_ref, head_ref)?.scoped_paths())
}

/// `git diff --name-status -z base_ref head_ref` → structured change set.
pub fn git_diff_name_status(repo_root: &Path, base_ref: &str, head_ref: &str) -> Result<ChangeSet> {
    let output = Command::new("git")
        .args(["diff", "--name-status", "-z", base_ref, head_ref])
        .current_dir(repo_root)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(format!("git diff failed: {stderr}")));
    }

    Ok(parse_name_status_z(&output.stdout))
}

/// Parse NUL-separated `git diff --name-status -z` output.
pub fn parse_name_status_z(stdout: &[u8]) -> ChangeSet {
    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut deleted = Vec::new();
    let mut renamed = Vec::new();

    let parts: Vec<&[u8]> = stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .collect();

    let mut idx = 0usize;
    while idx < parts.len() {
        let status = String::from_utf8_lossy(parts[idx]);
        let letter = status.chars().next();
        idx += 1;

        match letter {
            Some('A') => {
                if idx < parts.len() {
                    added.push(normalize_path_str(&String::from_utf8_lossy(parts[idx])));
                    idx += 1;
                }
            }
            Some('M') | Some('T') => {
                if idx < parts.len() {
                    changed.push(normalize_path_str(&String::from_utf8_lossy(parts[idx])));
                    idx += 1;
                }
            }
            Some('D') => {
                if idx < parts.len() {
                    deleted.push(normalize_path_str(&String::from_utf8_lossy(parts[idx])));
                    idx += 1;
                }
            }
            Some('R') | Some('C') => {
                if idx + 1 < parts.len() {
                    let old_path = normalize_path_str(&String::from_utf8_lossy(parts[idx]));
                    let new_path = normalize_path_str(&String::from_utf8_lossy(parts[idx + 1]));
                    renamed.push((old_path, new_path));
                    idx += 2;
                }
            }
            _ => {}
        }
    }

    ChangeSet {
        added,
        changed,
        deleted,
        renamed,
    }
}

/// `git diff -U0 base_ref head_ref` unified diff text.
pub fn git_unified_diff(repo_root: &Path, base_ref: &str, head_ref: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "-U0", base_ref, head_ref])
        .current_dir(repo_root)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(format!("git diff failed: {stderr}")));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// `git diff --name-status -z HEAD` — working tree vs `HEAD` commit.
pub fn git_diff_worktree_vs_head(repo_root: &Path) -> Result<ChangeSet> {
    let output = Command::new("git")
        .args(["diff", "--name-status", "-z", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        return Ok(ChangeSet::default());
    }
    Ok(parse_name_status_z(&output.stdout))
}

fn git_diff_worktree_name_only(repo_root: &Path) -> Result<Vec<String>> {
    Ok(git_diff_worktree_vs_head(repo_root)?.scoped_paths())
}

/// `git diff -U0 HEAD` unified diff for the working tree.
pub fn git_unified_diff_worktree(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "-U0", "HEAD"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Commits reachable from `head_ref` but not `base_ref`, oldest first.
pub fn git_rev_list_reverse(repo_root: &Path, base_ref: &str, head_ref: &str) -> Result<Vec<String>> {
    let range = format!("{base_ref}..{head_ref}");
    let output = Command::new("git")
        .args(["rev-list", "--reverse", &range])
        .current_dir(repo_root)
        .output()
        .map_err(|e| Error::Other(format!("Failed to run git: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(format!("git rev-list failed: {stderr}")));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// Resolve a snapshot path from a repo root or direct snapshot file.
pub fn resolve_snapshot_path(artifact: &Path) -> PathBuf {
    if artifact.is_file() {
        return artifact.to_path_buf();
    }
    MmappedGraphSnapshot::default_path(artifact)
}

/// Environment variable for the base graph artifact root (CI caches).
pub const RGCTL_BASE_ARTIFACT_ENV: &str = "RGCTL_BASE_ARTIFACT";

/// Conventional base artifact directory under a repository root (`{repo}/.rgctl-base/.rgctl/`).
pub const DEFAULT_BASE_ARTIFACT_SUBDIR: &str = ".rgctl-base";

/// Resolved mmap snapshot paths for `pr-check`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrCheckArtifactPaths {
    pub base_snapshot: PathBuf,
    pub head_snapshot: PathBuf,
}

/// Resolve base/head snapshot files with defaults.
///
/// Head defaults to `repo` (`{repo}/.rgctl/graph.snapshot.bin`).
/// Base resolves in order: `--base-artifact`, `$RGCTL_BASE_ARTIFACT`, `{repo}/.rgctl-base`.
pub fn resolve_pr_check_artifacts(
    repo: &Path,
    base_artifact: Option<&Path>,
    head_artifact: Option<&Path>,
) -> Result<PrCheckArtifactPaths> {
    let base_root = resolve_base_artifact_root(repo, base_artifact)?;
    let base_snapshot = resolve_snapshot_path(&base_root);
    ensure_snapshot_exists(&base_snapshot, "base")?;

    let head_root = head_artifact
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| repo.to_path_buf());
    let head_snapshot = resolve_snapshot_path(&head_root);
    ensure_snapshot_exists(&head_snapshot, "head")?;

    Ok(PrCheckArtifactPaths {
        base_snapshot,
        head_snapshot,
    })
}

/// Resolve the base snapshot path (no head existence check).
pub fn resolve_base_snapshot(
    repo: &Path,
    base_artifact: Option<&Path>,
) -> Result<PathBuf> {
    let base_root = resolve_base_artifact_root(repo, base_artifact)?;
    let base_snapshot = resolve_snapshot_path(&base_root);
    ensure_snapshot_exists(&base_snapshot, "base")?;
    Ok(base_snapshot)
}

/// Root directory for the base artifact (for delta head synthesis).
pub fn resolve_base_artifact_root(
    repo: &Path,
    base_artifact: Option<&Path>,
) -> Result<PathBuf> {
    resolve_base_artifact_root_impl(repo, base_artifact)
}

fn resolve_base_artifact_root_impl(repo: &Path, base_artifact: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = base_artifact {
        return Ok(path.to_path_buf());
    }
    if let Ok(env) = std::env::var(RGCTL_BASE_ARTIFACT_ENV) {
        if !env.is_empty() {
            return Ok(PathBuf::from(env));
        }
    }
    let conventional = repo.join(DEFAULT_BASE_ARTIFACT_SUBDIR);
    if conventional.exists() {
        return Ok(conventional);
    }
    Err(Error::Other(format!(
        "base graph snapshot not configured: pass --base-artifact, set {RGCTL_BASE_ARTIFACT_ENV}, or create {}/.rgctl/ (see docs/guides/ci-policy-checks.md)",
        DEFAULT_BASE_ARTIFACT_SUBDIR
    )))
}

fn ensure_snapshot_exists(path: &Path, label: &str) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }
    Err(Error::Other(format!(
        "{label} snapshot not found at {} (run `rgctl discover .` in the artifact tree first)",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgctl_graph::backend::GraphBackend;
    use rgctl_graph::schema::Node;
    use std::fs;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        for args in [
            ["init", "-b", "main"],
            ["config", "user.email", "test@example.com"],
            ["config", "user.name", "test"],
        ] {
            let out = Command::new("git")
                .args(args)
                .current_dir(dir)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .output()
                .unwrap();
            assert!(out.status.success(), "git {:?} failed", args);
        }
    }

    fn git_commit_all(dir: &Path, message: &str) {
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        let out = Command::new("git")
            .args(["-c", "commit.gpgsign=false", "commit", "-m", message])
            .current_dir(dir)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn hunk_index_parses_new_side_ranges() {
        let diff = "\
diff --git a/src/a.rs b/src/a.rs
index 111..222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -10,3 +10,4 @@ fn foo() {
";
        let index = HunkIndex::from_unified_diff(diff);
        let ranges = index.ranges_for("src/a.rs").expect("ranges");
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, 10);
        assert_eq!(ranges[0].end, 13);
    }

    #[test]
    fn scoped_paths_from_git_fixture() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/a.rs"), "fn old() {}\n").unwrap();
        git_commit_all(tmp.path(), "base");

        fs::write(tmp.path().join("src/a.rs"), "fn new() {}\n").unwrap();
        fs::write(tmp.path().join("src/b.rs"), "fn b() {}\n").unwrap();
        git_commit_all(tmp.path(), "head");

        let paths = ScopedPaths::from_git_refs(tmp.path(), "HEAD~1", "HEAD").unwrap();
        assert!(paths.contains_path("src/a.rs"));
        assert!(paths.contains_path("src/b.rs"));
        assert!(!paths.contains_path("missing.rs"));
        let change_set = paths.change_set();
        assert_eq!(change_set.added.len(), 1);
        assert_eq!(change_set.changed.len(), 1);
        assert!(change_set.added.contains(&"src/b.rs".to_string()));
        assert!(change_set.changed.contains(&"src/a.rs".to_string()));
    }

    #[test]
    fn git_name_status_fixture_add_modify_delete_rename() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/keep.rs"), "fn keep() {}\n").unwrap();
        fs::write(tmp.path().join("src/modify.rs"), "fn old() {}\n").unwrap();
        fs::write(tmp.path().join("src/remove.rs"), "fn gone() {}\n").unwrap();
        fs::write(tmp.path().join("src/old_name.rs"), "fn renamed() {}\n").unwrap();
        git_commit_all(tmp.path(), "base");

        fs::write(tmp.path().join("src/modify.rs"), "fn new() {}\n").unwrap();
        fs::write(tmp.path().join("src/added.rs"), "fn added() {}\n").unwrap();
        fs::remove_file(tmp.path().join("src/remove.rs")).unwrap();
        fs::rename(
            tmp.path().join("src/old_name.rs"),
            tmp.path().join("src/new_name.rs"),
        )
        .unwrap();
        git_commit_all(tmp.path(), "head");

        let changes = git_diff_name_status(tmp.path(), "HEAD~1", "HEAD").unwrap();
        assert!(changes.added.contains(&"src/added.rs".to_string()));
        assert!(changes.changed.contains(&"src/modify.rs".to_string()));
        assert!(changes.deleted.contains(&"src/remove.rs".to_string()));
        assert_eq!(
            changes.renamed,
            vec![("src/old_name.rs".to_string(), "src/new_name.rs".to_string())]
        );

        let scoped = ScopedPaths::from_change_set(changes);
        assert!(scoped.contains_path("src/added.rs"));
        assert!(scoped.contains_path("src/modify.rs"));
        assert!(scoped.contains_path("src/remove.rs"));
        assert!(scoped.contains_path("src/new_name.rs"));
        assert!(scoped.invalidation_paths().contains(&"src/remove.rs".to_string()));
        assert!(scoped.invalidation_paths().contains(&"src/old_name.rs".to_string()));
        assert!(!scoped.invalidation_paths().contains(&"src/added.rs".to_string()));
    }

    #[test]
    fn parse_name_status_z_handles_all_status_letters() {
        let raw = b"M\0src/mod.rs\0A\0src/new.rs\0D\0src/old.rs\0R100\0src/from.rs\0src/to.rs\0";
        let changes = parse_name_status_z(raw);
        assert_eq!(changes.changed, vec!["src/mod.rs".to_string()]);
        assert_eq!(changes.added, vec!["src/new.rs".to_string()]);
        assert_eq!(changes.deleted, vec!["src/old.rs".to_string()]);
        assert_eq!(
            changes.renamed,
            vec![("src/from.rs".to_string(), "src/to.rs".to_string())]
        );
    }

    #[test]
    fn changed_function_symbols_strict_empty_diff_errors() {
        let tmp = TempDir::new().unwrap();
        init_git_repo(tmp.path());
        fs::write(tmp.path().join("f.rs"), "fn x() {}\n").unwrap();
        git_commit_all(tmp.path(), "only");

        let mut backend = MemoryBackend::new();
        let node = Node::new(NodeType::Function, "x").with_file_path("f.rs");
        backend.insert_node(node).unwrap();

        let err = changed_function_symbols(
            tmp.path(),
            &backend,
            &SymbolScopeOptions {
                base_ref: Some("HEAD".into()),
                head_ref: Some("HEAD".into()),
                strict: true,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("strict"));
    }

    #[test]
    fn resolve_pr_check_artifacts_defaults_head_to_repo() {
        let tmp = TempDir::new().unwrap();
        let rgctl = tmp.path().join(".rgctl");
        fs::create_dir_all(&rgctl).unwrap();
        fs::write(rgctl.join("graph.snapshot.bin"), b"snap").unwrap();

        let err = resolve_pr_check_artifacts(tmp.path(), None, None).unwrap_err();
        assert!(err.to_string().contains("base graph snapshot not configured"));

        let base = tmp.path().join(DEFAULT_BASE_ARTIFACT_SUBDIR);
        fs::create_dir_all(base.join(".rgctl")).unwrap();
        fs::write(base.join(".rgctl/graph.snapshot.bin"), b"base").unwrap();

        let paths = resolve_pr_check_artifacts(tmp.path(), None, None).unwrap();
        assert!(paths.head_snapshot.ends_with("graph.snapshot.bin"));
        assert!(paths.base_snapshot.ends_with("graph.snapshot.bin"));
    }

    #[test]
    fn resolve_pr_check_artifacts_explicit_override() {
        let tmp = TempDir::new().unwrap();
        let head = tmp.path().join("head");
        fs::create_dir_all(head.join(".rgctl")).unwrap();
        fs::write(head.join(".rgctl/graph.snapshot.bin"), b"head").unwrap();
        let base = tmp.path().join("base");
        fs::create_dir_all(base.join(".rgctl")).unwrap();
        fs::write(base.join(".rgctl/graph.snapshot.bin"), b"base").unwrap();

        let paths = resolve_pr_check_artifacts(tmp.path(), Some(&base), Some(&head)).unwrap();
        assert!(paths.base_snapshot.to_string_lossy().contains("/base/"));
        assert!(paths.head_snapshot.to_string_lossy().contains("/head/"));
    }

    #[test]
    fn function_symbols_in_paths_suffix_match() {
        let mut backend = MemoryBackend::new();
        backend
            .insert_node(Node::new(NodeType::Function, "foo").with_file_path("src/foo.rs"))
            .unwrap();
        let symbols = function_symbols_in_paths(&backend, &["foo.rs".into()]);
        assert_eq!(symbols, vec!["foo".to_string()]);
    }
}
