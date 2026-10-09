//! `rgctl update` — incremental structural graph patch (not full discover).

use super::context::CliContext;
use super::OutputFormat;
use crate::discovery::DiscoveryConfig;
use crate::languages::registry::LanguageRegistry;
use anyhow::{Context, Result};
use rgctl_graph::code_graph::CodeGraph;
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_incremental::{IncrementalUpdater, UpdateOptions, UpdateResult};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const UPDATE_SCHEMA_VERSION: u32 = 1;

/// Arguments for `rgctl update`.
#[derive(Debug, Clone)]
pub struct UpdateArgs {
    /// Optional session path (defaults to `--repo` / cwd).
    pub path: Option<String>,
    /// Explicit repo-relative paths (CSV via clap).
    pub files: Option<Vec<String>>,
    /// Git ref for `git diff --name-only` change set.
    pub since: Option<String>,
    /// Force full rebuild via updater.
    pub force: bool,
    /// Reverse CALLS cascade hops.
    pub cascade_depth: usize,
    pub languages: Option<String>,
    pub exclude: Option<String>,
}

#[derive(Debug, Serialize)]
struct UpdateJson {
    schema_version: u32,
    command: &'static str,
    files_added: usize,
    files_changed: usize,
    files_deleted: usize,
    files_affected: usize,
    nodes_added: usize,
    nodes_removed: usize,
    edges_added: usize,
    edges_removed: usize,
    duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// Run incremental update for the session repo.
pub fn run(ctx: &CliContext, args: UpdateArgs) -> Result<()> {
    let root = PathBuf::from(super::discover::resolve_session_root(
        ctx,
        args.path.as_deref(),
    ));
    let result = run_update_at(&root, &args, ctx.format != OutputFormat::Json, true)?;
    emit_result(ctx, &result)?;
    Ok(())
}

/// Shared entry used by CLI and `serve --watch`.
///
/// When `acquire_lock` is true (CLI `rgctl update`), takes `.rgctl/watch.lock` so it
/// cannot race a concurrent `serve --watch`. The watch loop passes `false` because
/// serve already holds that lock for the process lifetime.
pub fn run_update_at(
    root: &Path,
    args: &UpdateArgs,
    show_progress: bool,
    acquire_lock: bool,
) -> Result<UpdateResult> {
    let _lock = if acquire_lock {
        Some(
            super::pipeline_status::try_acquire_watch_lock(root)
                .context("cannot run rgctl update while another watch/update holds the lock")?,
        )
    } else {
        None
    };

    let snapshot = MmappedGraphSnapshot::default_path(root);
    if !snapshot.is_file() {
        anyhow::bail!(
            "no graph snapshot at {}; run `rgctl discover` first",
            snapshot.display()
        );
    }

    let discovery = discovery_from_args(args);
    let registry: Arc<rgctl_registry::LanguageRegistry> = LanguageRegistry::new().into();
    let mut graph = CodeGraph::open_snapshot(&snapshot)
        .with_context(|| format!("open snapshot {}", snapshot.display()))?;

    let updater = IncrementalUpdater::with_options(
        registry,
        UpdateOptions {
            since: args.since.clone(),
            force: args.force,
            discovery,
            show_progress,
            cascade_depth: args.cascade_depth,
            ..Default::default()
        },
    );

    if let Some(files) = &args.files {
        updater
            .update_files(&mut graph, root, files)
            .with_context(|| "incremental file update")
    } else {
        updater
            .update(&mut graph, root)
            .with_context(|| "incremental graph update")
    }
}

/// Incremental update for an explicit path list (watch path; lock already held by serve).
pub fn update_paths(root: &Path, relative_paths: &[String], cascade_depth: usize) -> Result<UpdateResult> {
    let args = UpdateArgs {
        path: None,
        files: Some(relative_paths.to_vec()),
        since: None,
        force: false,
        cascade_depth,
        languages: None,
        exclude: None,
    };
    run_update_at(root, &args, false, false)
}

fn discovery_from_args(args: &UpdateArgs) -> DiscoveryConfig {
    let mut discovery = DiscoveryConfig::default();
    if let Some(langs) = &args.languages {
        discovery.languages = Some(
            langs
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        );
    }
    if let Some(excludes) = &args.exclude {
        discovery.exclude_patterns = excludes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    discovery
}

fn emit_result(ctx: &CliContext, result: &UpdateResult) -> Result<()> {
    let affected = result.files_affected();
    let message = if affected == 0 {
        Some("already current".to_string())
    } else {
        None
    };

    if ctx.format == OutputFormat::Json {
        let payload = UpdateJson {
            schema_version: UPDATE_SCHEMA_VERSION,
            command: "update",
            files_added: result.files_added,
            files_changed: result.files_changed,
            files_deleted: result.files_deleted,
            files_affected: affected,
            nodes_added: result.nodes_added,
            nodes_removed: result.nodes_removed,
            edges_added: result.edges_added,
            edges_removed: result.edges_removed,
            duration_ms: duration_ms(result.duration),
            message,
        };
        ctx.emit_json_value(&serde_json::to_value(&payload)?)?;
    } else if affected == 0 {
        println!("Graph already current (no files changed)");
    } else {
        println!(
            "Updated {} files (+{}/-{} nodes, +{}/-{} edges) in {:.2}s",
            affected,
            result.nodes_added,
            result.nodes_removed,
            result.edges_added,
            result.edges_removed,
            result.duration.as_secs_f64()
        );
    }
    Ok(())
}

fn duration_ms(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}
