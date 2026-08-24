//! `rg-build discover` — index and analyze a repository.

use super::context::CliContext;
use super::discover_impl::{AnalysisOptions, run_full_analysis};
use super::pipeline_session::{FullPipelineArgs, run_full_pipeline};
use anyhow::Result;

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
    /// Export `.rgbuilder/dashboard/` bundle. Default off.
    pub with_dashboard: bool,
    /// Write a migration roadmap JSON after analysis completes.
    pub export_migration_hints: bool,
    /// Compute harmonic centrality (HyperBall on large graphs). Default off.
    pub with_harmonic: bool,
    /// Staged full pipeline (`--full`).
    pub full: bool,
    /// Preset strategy for `--export-migration-hints` (default: hybrid_default).
    pub migration_preset: String,
    /// Roadmap row order: `scheduled` (deps) or `priority` (score rank).
    pub migration_order: String,
    /// When set, persist artifacts here instead of the scanned tree (daemon cache).
    pub artifact_root: Option<std::path::PathBuf>,
}

/// Resolve discover root: absolute PATH, PATH joined to `--repo`, or `--repo`/cwd.
pub fn resolve_session_root(ctx: &CliContext, path: Option<&str>) -> String {
    path.map(|p| {
        if std::path::Path::new(p).is_absolute() {
            p.to_string()
        } else {
            ctx.repo.join(p).to_string_lossy().into_owned()
        }
    })
    .unwrap_or_else(|| ctx.repo.to_string_lossy().into_owned())
}

pub fn run(ctx: &CliContext, args: DiscoverArgs) -> Result<()> {
    if args.artifact_root.is_none() && super::daemon::route_discover(ctx, args.clone())? {
        return Ok(());
    }
    let path = resolve_session_root(ctx, args.path.as_deref());

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
