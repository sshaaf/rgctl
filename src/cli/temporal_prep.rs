//! Shared base/head snapshot preparation for `review check` / `review paths` / `pr-check`.

use super::context::CliContext;
use crate::languages::registry::LanguageRegistry;
use anyhow::{Context, Result};
use rgctl_graph::code_graph::GRAPH_DIR;
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_incremental::{
    HeadSynthesisOptions, HunkIndex, PrCheckArtifactPaths, ScopedPaths, git_diff_worktree_vs_head,
    git_unified_diff, git_unified_diff_worktree, resolve_base_artifact_root,
    resolve_pr_check_artifacts, synthesize_head_snapshot, synthesize_worktree_head_snapshot,
};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntheticHeadMode {
    None,
    Worktree,
}

impl SyntheticHeadMode {
    pub fn parse(value: Option<&str>) -> Result<Self> {
        match value {
            None => Ok(Self::None),
            Some("worktree") => Ok(Self::Worktree),
            Some(other) => anyhow::bail!(
                "unsupported --synthetic-head value '{other}' (supported: worktree)"
            ),
        }
    }
}

/// Shared temporal artifact / ref selection (no policy fields).
#[derive(Clone, Debug)]
pub struct TemporalArtifactArgs {
    pub base_artifact: Option<String>,
    pub head_artifact: Option<String>,
    pub base_ref: String,
    pub head_ref: String,
    pub cascade_depth: usize,
    pub full_snapshots: bool,
}

pub fn prepare_artifacts(
    ctx: &CliContext,
    args: &TemporalArtifactArgs,
    synthetic_head: SyntheticHeadMode,
) -> Result<PrCheckArtifactPaths> {
    if synthetic_head == SyntheticHeadMode::Worktree {
        let registry: Arc<rgctl_registry::LanguageRegistry> = LanguageRegistry::new().into();
        synthesize_worktree_head_snapshot(&ctx.repo, args.cascade_depth, registry)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let base_snapshot = rgctl_incremental::resolve_base_snapshot(
            &ctx.repo,
            args.base_artifact.as_deref().map(Path::new),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let head_snapshot = MmappedGraphSnapshot::default_path(&ctx.repo);
        return Ok(PrCheckArtifactPaths {
            base_snapshot,
            head_snapshot,
        });
    }

    let delta_head = !args.full_snapshots && args.head_artifact.is_none();
    if delta_head {
        let base_root = resolve_base_artifact_root(
            &ctx.repo,
            args.base_artifact.as_deref().map(Path::new),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let registry: Arc<rgctl_registry::LanguageRegistry> = LanguageRegistry::new().into();
        synthesize_head_snapshot(
            &base_root,
            &ctx.repo,
            &HeadSynthesisOptions {
                base_ref: args.base_ref.clone(),
                head_ref: args.head_ref.clone(),
                cascade_depth: args.cascade_depth,
            },
            registry,
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let base_snapshot = rgctl_incremental::resolve_base_snapshot(
            &ctx.repo,
            args.base_artifact.as_deref().map(Path::new),
        )
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        let head_snapshot = MmappedGraphSnapshot::default_path(&ctx.repo);
        Ok(PrCheckArtifactPaths {
            base_snapshot,
            head_snapshot,
        })
    } else {
        resolve_pr_check_artifacts(
            &ctx.repo,
            args.base_artifact.as_deref().map(Path::new),
            args.head_artifact.as_deref().map(Path::new),
        )
        .map_err(|e| {
            anyhow::anyhow!(
                "{e}; prepare base/head graph snapshots (e.g. discover into `.rgctl-base` / `.rgctl`) \
                 or pass `--base-artifact` / `--head-artifact`, or omit `--full-snapshots` to synthesize a delta head"
            )
        })
    }
}

pub fn resolve_scope_paths(
    ctx: &CliContext,
    args: &TemporalArtifactArgs,
    synthetic_head: SyntheticHeadMode,
) -> Result<ScopedPaths> {
    if synthetic_head == SyntheticHeadMode::Worktree {
        let change_set = git_diff_worktree_vs_head(&ctx.repo)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        return Ok(ScopedPaths::from_change_set(change_set));
    }
    ScopedPaths::from_git_refs(&ctx.repo, &args.base_ref, &args.head_ref)
        .with_context(|| "resolve git diff paths")
}

pub fn resolve_unified_diff(
    ctx: &CliContext,
    args: &TemporalArtifactArgs,
    synthetic_head: SyntheticHeadMode,
) -> Result<String> {
    if synthetic_head == SyntheticHeadMode::Worktree {
        return git_unified_diff_worktree(&ctx.repo).map_err(|e| anyhow::anyhow!(e.to_string()));
    }
    git_unified_diff(&ctx.repo, &args.base_ref, &args.head_ref)
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

pub fn resolve_hunk_index(
    ctx: &CliContext,
    args: &TemporalArtifactArgs,
    synthetic_head: SyntheticHeadMode,
) -> Result<HunkIndex> {
    let unified = resolve_unified_diff(ctx, args, synthetic_head)?;
    Ok(HunkIndex::from_unified_diff(&unified))
}

/// Default session graph dir helper (unused by paths; kept for callers).
#[allow(dead_code)]
pub fn session_graph_dir(repo: &Path) -> std::path::PathBuf {
    repo.join(GRAPH_DIR)
}
