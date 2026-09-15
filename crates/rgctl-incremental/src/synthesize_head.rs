//! Delta head snapshot synthesis from a cached base artifact + git change set.

use crate::file_tracker::FILE_HASHES_FILE;
use crate::pr_scope::{git_diff_name_status, git_diff_worktree_vs_head, resolve_snapshot_path};
use crate::updater::{IncrementalUpdater, UpdateOptions};
use rgctl_error::Result;
use rgctl_graph::code_graph::{CodeGraph, GRAPH_DIR};
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_registry::LanguageRegistry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Git refs and compaction options for head synthesis.
#[derive(Debug, Clone)]
pub struct HeadSynthesisOptions {
    pub base_ref: String,
    pub head_ref: String,
    pub cascade_depth: usize,
}

/// Copy base `.rgctl/` snapshot (and file hashes when present) into `repo_root/.rgctl/`.
pub fn seed_head_artifact_from_base(base_artifact_root: &Path, repo_root: &Path) -> Result<()> {
    let base_snapshot = resolve_snapshot_path(base_artifact_root);
    let base_rgctl = rgctl_dir_for_artifact(base_artifact_root);
    let head_rgctl = repo_root.join(GRAPH_DIR);
    std::fs::create_dir_all(&head_rgctl)?;

    let head_snapshot = MmappedGraphSnapshot::default_path(repo_root);
    std::fs::copy(&base_snapshot, &head_snapshot)?;

    let hashes = base_rgctl.join(FILE_HASHES_FILE);
    if hashes.is_file() {
        std::fs::copy(&hashes, head_rgctl.join(FILE_HASHES_FILE))?;
    }
    Ok(())
}

/// Build a head columnar snapshot at `{repo}/.rgctl/graph.snapshot.bin` from base + git delta.
pub fn synthesize_head_snapshot(
    base_artifact_root: &Path,
    repo_root: &Path,
    opts: &HeadSynthesisOptions,
    registry: Arc<LanguageRegistry>,
) -> Result<PathBuf> {
    seed_head_artifact_from_base(base_artifact_root, repo_root)?;
    let changes = git_diff_name_status(repo_root, &opts.base_ref, &opts.head_ref)?;
    let head_snapshot = MmappedGraphSnapshot::default_path(repo_root);
    let mut graph = CodeGraph::open_snapshot(&head_snapshot)?;
    let updater = IncrementalUpdater::with_options(
        registry,
        UpdateOptions {
            cascade_depth: opts.cascade_depth,
            show_progress: false,
            ..Default::default()
        },
    );
    updater.apply_change_set(&mut graph, repo_root, changes)?;
    Ok(head_snapshot)
}

/// Compact the committed `HEAD` snapshot with working-tree file changes.
pub fn synthesize_worktree_head_snapshot(
    repo_root: &Path,
    cascade_depth: usize,
    registry: Arc<LanguageRegistry>,
) -> Result<PathBuf> {
    let head_snapshot = MmappedGraphSnapshot::default_path(repo_root);
    if !head_snapshot.is_file() {
        return Err(rgctl_error::Error::Other(
            "worktree head synthesis requires an existing HEAD snapshot at .rgctl/graph.snapshot.bin (run `rgctl discover` first)".into(),
        ));
    }
    let changes = git_diff_worktree_vs_head(repo_root)?;
    let mut graph = CodeGraph::open_snapshot(&head_snapshot)?;
    let updater = IncrementalUpdater::with_options(
        registry,
        UpdateOptions {
            cascade_depth,
            show_progress: false,
            ..Default::default()
        },
    );
    updater.apply_change_set(&mut graph, repo_root, changes)?;
    Ok(head_snapshot)
}

fn rgctl_dir_for_artifact(artifact_root: &Path) -> PathBuf {
    if artifact_root
        .file_name()
        .is_some_and(|n| n == GRAPH_DIR)
    {
        return artifact_root.to_path_buf();
    }
    if artifact_root.join(GRAPH_DIR).is_dir() {
        return artifact_root.join(GRAPH_DIR);
    }
    resolve_snapshot_path(artifact_root)
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| artifact_root.to_path_buf())
}
