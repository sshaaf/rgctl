//! Export community-level migration graph for the dashboard bundle.

use crate::export_context::{DashboardExportContext, resolve_analysis};
use rgctl_analysis::{
    MigrationGraphPayload, MigrationOrderMode, MigrationPlanPayload, MigrationWeights,
    build_migration_graph, compute_migration_plan,
};
use rgctl_graph::backend::MemoryBackend;
use std::fs;
use std::path::Path;

pub const MIGRATION_GRAPH_FILE: &str = "migration_graph.json";
pub const MIGRATION_PLAN_FILE: &str = "migration_plan.json";

#[derive(Debug, Clone, Default)]
pub struct MigrationExportSummary {
    pub available: bool,
    pub community_count: usize,
    pub edge_count: usize,
}

pub fn export_migration_graph(
    backend: &MemoryBackend,
    repo_root: &Path,
    out_dir: &Path,
    ctx: DashboardExportContext<'_>,
) -> Result<(MigrationExportSummary, Option<MigrationGraphPayload>), String> {
    let results = match resolve_analysis(&ctx, repo_root) {
        Ok(cow) => cow,
        Err(_) => {
            write_empty_graph(out_dir)?;
            return Ok((MigrationExportSummary::default(), None));
        }
    };

    let Some(graph) = build_migration_graph(backend, &results) else {
        write_empty_graph(out_dir)?;
        return Ok((MigrationExportSummary::default(), None));
    };

    let json = serde_json::to_string_pretty(&graph).map_err(|e| e.to_string())?;
    fs::write(out_dir.join(MIGRATION_GRAPH_FILE), json).map_err(|e| e.to_string())?;

    let summary = MigrationExportSummary {
        available: true,
        community_count: graph.communities.len(),
        edge_count: graph.edges.len(),
    };
    Ok((summary, Some(graph)))
}

pub fn export_default_migration_plan(
    graph: &MigrationGraphPayload,
    out_dir: &Path,
) -> Result<(), String> {
    let plan = compute_migration_plan(
        graph,
        "hybrid_default",
        MigrationWeights::hybrid_default(),
        MigrationOrderMode::Scheduled,
    );
    write_migration_plan(&plan, &out_dir.join(MIGRATION_PLAN_FILE))
}

pub fn write_migration_plan(plan: &MigrationPlanPayload, path: &Path) -> Result<(), String> {
    let json = serde_json::to_string_pretty(plan).map_err(|e| e.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, json).map_err(|e| e.to_string())
}

pub fn write_migration_plan_from_repo(
    backend: &MemoryBackend,
    repo_root: &Path,
    output: &Path,
    preset: &str,
    order_mode: MigrationOrderMode,
) -> Result<MigrationPlanPayload, String> {
    write_migration_plan_from_repo_with_context(
        backend,
        repo_root,
        output,
        preset,
        order_mode,
        DashboardExportContext::default(),
    )
}

pub fn write_migration_plan_from_repo_with_context(
    backend: &MemoryBackend,
    repo_root: &Path,
    output: &Path,
    preset: &str,
    order_mode: MigrationOrderMode,
    ctx: DashboardExportContext<'_>,
) -> Result<MigrationPlanPayload, String> {
    let results = resolve_analysis(&ctx, repo_root)?;
    let graph = build_migration_graph(backend, &results)
        .ok_or_else(|| "migration graph unavailable (run discover first)".to_string())?;
    let weights = MigrationWeights::from_preset(preset);
    let plan = compute_migration_plan(&graph, preset, weights, order_mode);
    write_migration_plan(&plan, output)?;
    Ok(plan)
}

fn write_empty_graph(out_dir: &Path) -> Result<(), String> {
    let payload = MigrationGraphPayload {
        schema_version: rgctl_analysis::MIGRATION_GRAPH_SCHEMA_VERSION,
        mode: "package_macro".into(),
        modularity: 0.0,
        communities: Vec::new(),
        edges: Vec::new(),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    fs::write(out_dir.join(MIGRATION_GRAPH_FILE), json).map_err(|e| e.to_string())
}
