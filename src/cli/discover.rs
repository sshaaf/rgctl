//! `rgctl discover` — index and analyze a repository.

use super::args::OutputFormat;
use super::context::CliContext;
use super::discover_impl::{AnalysisOptions, run_full_analysis};
use super::pipeline_session::{FullPipelineArgs, run_full_pipeline};
use crate::discovery::DiscoveryConfig;
use crate::languages::registry::LanguageRegistry;
use anyhow::{Context, Result};
use rgctl_graph::code_graph::CodeGraph;
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_incremental::{IncrementalUpdater, UpdateOptions};
use std::path::Path;
use std::sync::Arc;

#[derive(Clone)]
pub struct DiscoverArgs {
    pub path: Option<String>,
    pub languages: Option<String>,
    pub exclude: Option<String>,
    /// Secret scanning. Default off.
    pub with_security: bool,
    /// CFG / dominators / PDG. Default off.
    pub with_cfg: bool,
    /// Discover-time taint (implies CFG pass). Default off.
    pub with_taint: bool,
    /// Classify loop-carried PDG data deps (implies CFG). Default off.
    pub with_dfg_loops: bool,
    /// Write coarse AST skeleton archive (implies CFG). Default off.
    pub with_ast_skeleton: bool,
    /// Also write legacy JSON graph files (`graph.db` / `graph.json`).
    pub write_json_graph: bool,
    /// Export `.rgctl/dashboard/` bundle. Default off.
    pub with_dashboard: bool,
    /// Write a migration roadmap JSON after analysis completes.
    pub export_migration_hints: bool,
    /// Compute harmonic centrality (HyperBall on large graphs). Default off.
    pub with_harmonic: bool,
    /// Native Kantra rule evaluation during discover.
    pub with_kantra: bool,
    /// Override embedded catalog with a single ruleset directory.
    pub kantra_rules: Option<String>,
    /// Override embedded catalog with a ruleset tree (`ruleset.yaml` dirs).
    pub kantra_catalog: Option<String>,
    /// Evaluate only rules with `konveyor.io/target=<name>`.
    pub kantra_target: Option<String>,
    /// Hydrate Kantra rule nodes only; skip violation eval.
    pub kantra_index_only: bool,
    /// Staged full pipeline (`--full`).
    pub full: bool,
    /// Preset strategy for `--export-migration-hints` (default: hybrid_default).
    pub migration_preset: String,
    /// Roadmap row order: `scheduled` (deps) or `priority` (score rank).
    pub migration_order: String,
    /// When set, persist artifacts here instead of the scanned tree (daemon cache).
    pub artifact_root: Option<std::path::PathBuf>,
    /// Incremental file paths (repo-relative); skips full discover when set.
    pub files: Option<Vec<String>>,
    /// Reverse call-dependency hops for `--files` updates.
    pub cascade_depth: usize,
}

/// Resolve discover root: absolute PATH, PATH joined to `--repo`, or `--repo`/cwd.
pub fn resolve_session_root(ctx: &CliContext, path: Option<&str>) -> String {
    let raw = path.map(|p| {
        if std::path::Path::new(p).is_absolute() {
            p.to_string()
        } else {
            ctx.repo.join(p).to_string_lossy().into_owned()
        }
    })
    .unwrap_or_else(|| ctx.repo.to_string_lossy().into_owned());
    let p = std::path::PathBuf::from(&raw);
    p.canonicalize()
        .or_else(|_| std::env::current_dir().map(|cwd| cwd.join(&p)))
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

pub fn run(ctx: &CliContext, args: DiscoverArgs) -> Result<()> {
    super::kantra_discover::validate_kantra_flags(
        args.with_kantra,
        &args.kantra_rules,
        &args.kantra_catalog,
    )?;
    let path = resolve_session_root(ctx, args.path.as_deref());

    if let Some(files) = &args.files {
        return run_files_update(ctx, &path, files.clone(), &args);
    }

    if args.full {
        run_full_pipeline(ctx, &path, FullPipelineArgs::from_discover(&args))?;
        return Ok(());
    }

    let _ = run_full_analysis(
        ctx,
        &path,
        AnalysisOptions {
            languages: args.languages,
            exclude: args.exclude,
            with_security: args.with_security,
            with_cfg: args.with_cfg,
            with_taint: args.with_taint,
            with_dfg_loops: args.with_dfg_loops,
            with_ast_skeleton: args.with_ast_skeleton,
            write_json_graph: args.write_json_graph,
            with_dashboard: args.with_dashboard,
            export_migration_hints: args.export_migration_hints,
            with_harmonic: args.with_harmonic,
            with_kantra: args.with_kantra,
            kantra_rules: args.kantra_rules.clone(),
            kantra_catalog: args.kantra_catalog.clone(),
            kantra_target: args.kantra_target.clone(),
            kantra_index_only: args.kantra_index_only,
            migration_preset: &args.migration_preset,
            migration_order: &args.migration_order,
            db_path: &ctx.db,
            force_materialize_fields: false,
            force_reindex: false,
            emit_cli_summary: true,
            artifact_root: args.artifact_root.as_deref(),
        },
    )?;
    Ok(())
}

fn run_files_update(ctx: &CliContext, path: &str, files: Vec<String>, args: &DiscoverArgs) -> Result<()> {
    let root = Path::new(path);
    let snapshot = MmappedGraphSnapshot::default_path(root);
    if !snapshot.is_file() {
        anyhow::bail!(
            "no graph snapshot at {}; run `rgctl discover` first",
            snapshot.display()
        );
    }

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

    let registry: Arc<rgctl_registry::LanguageRegistry> = LanguageRegistry::new().into();
    let mut graph = CodeGraph::open_snapshot(&snapshot)
        .with_context(|| format!("open snapshot {}", snapshot.display()))?;
    let updater = IncrementalUpdater::with_options(
        registry,
        UpdateOptions {
            discovery,
            cascade_depth: args.cascade_depth,
            show_progress: ctx.format != OutputFormat::Json,
            ..Default::default()
        },
    );
    let result = updater
        .update_files(&mut graph, root, &files)
        .with_context(|| "incremental file update")?;

    if ctx.format == OutputFormat::Json {
        ctx.emit_json_value(&serde_json::json!({
            "schema_version": 1,
            "files_affected": result.files_affected(),
            "nodes_added": result.nodes_added,
            "nodes_removed": result.nodes_removed,
        }))?;
    } else {
        println!(
            "Updated {} files (+{} / -{} nodes)",
            result.files_affected(),
            result.nodes_added,
            result.nodes_removed
        );
    }
    Ok(())
}
