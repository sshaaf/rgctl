//! Kantra discover stage integration.

use crate::analysis::graph_utils::PetGraphView;
use crate::analysis::ColdMetadataDb;
use crate::cli::stage_profile::DiscoverStageReport;
use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use rgctl_graph::schema::{EdgeType, Node, NodeType};
use rgctl_graph::snapshot::SNAPSHOT_FILE;
use rgctl_kantra::eval::filecontent::SourceCache;
use rgctl_kantra::loader::KantraRuleset;
use rgctl_kantra::{
    EvalContext, EvalEdge, EvalGraph, EvalNode, KantraCatalog, KantraEngine,
    rewrite_snapshot_with_catalog,
};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const TEXT_EXTENSIONS: &[&str] = &[
    ".go", ".java", ".kt", ".scala", ".py", ".rs", ".js", ".ts", ".tsx", ".jsx", ".c", ".h",
    ".cpp", ".hpp", ".cs", ".md", ".mdx", ".yaml", ".yml", ".xml", ".properties", ".gradle",
    ".kts", ".sql",
];

/// Resolve the rules catalog for index/eval (embedded, tree, or single ruleset dir).
pub fn resolve_kantra_catalog(
    rules_path: Option<&Path>,
    catalog_path: Option<&Path>,
) -> Result<KantraCatalog> {
    if let Some(path) = rules_path {
        let rs = KantraRuleset::load(path).map_err(|e| anyhow::anyhow!("kantra ruleset: {e}"))?;
        return Ok(KantraCatalog {
            catalog_id: format!("dir@{}", path.display()),
            name: rs.doc.name,
            description: rs.doc.description,
            rules: rs.doc.rules,
        });
    }
    if let Some(root) = catalog_path {
        return KantraCatalog::load_tree(root).map_err(|e| anyhow::anyhow!("kantra catalog: {e}"));
    }
    KantraCatalog::embedded().map_err(|e| anyhow::anyhow!("kantra embedded catalog: {e}"))
}

/// Hydrate `KantraRule` nodes into `graph.snapshot.bin`.
pub fn run_kantra_index(
    store: &Path,
    rules_path: Option<&Path>,
    catalog_path: Option<&Path>,
    profile: &mut DiscoverStageReport,
) -> Result<()> {
    let start = Instant::now();
    let catalog = resolve_kantra_catalog(rules_path, catalog_path)?;
    let snapshot_path = rgctl_graph::paths::artifact_path(store, SNAPSHOT_FILE);
    rewrite_snapshot_with_catalog(&snapshot_path, &catalog)
        .map_err(|e| anyhow::anyhow!("kantra index: {e}"))?;
    profile.kantra_index.secs = start.elapsed().as_secs_f64();
    Ok(())
}

/// Run Kantra evaluation and write `.rgctl/kantra_findings.json`.
pub fn run_kantra_stage(
    repo_root: &Path,
    store: &Path,
    rules_path: Option<&Path>,
    catalog_path: Option<&Path>,
    kantra_target: Option<&str>,
    files: &[PathBuf],
    cold: &ColdMetadataDb,
    petgraph_view: &PetGraphView,
    profile: &mut DiscoverStageReport,
) -> Result<rgctl_kantra::KantraFindings> {
    let kantra_start = Instant::now();
    let catalog = resolve_kantra_catalog(rules_path, catalog_path)?;
    let (engine, load_secs) = KantraEngine::from_catalog(catalog, kantra_target)
        .map_err(|e| anyhow::anyhow!("kantra catalog: {e}"))?;
    profile.kantra_load.secs = load_secs;

    let preload_start = Instant::now();
    let sources = preload_discovered_sources(repo_root, files, None);
    profile.kantra_filecontent.secs = preload_start.elapsed().as_secs_f64();

    let graph = build_eval_graph(cold, petgraph_view)?;
    let ctx = EvalContext {
        repo_root,
        files,
        sources: &sources,
        graph: &graph,
    };

    let ref_start = Instant::now();
    let (findings, timings) = engine
        .evaluate(&ctx)
        .map_err(|e| anyhow::anyhow!("kantra evaluate: {e}"))?;
    profile.kantra_referenced.secs = ref_start.elapsed().as_secs_f64();
    profile.kantra_compose.secs = timings.compose_secs;
    profile.kantra_filecontent.secs += timings.filecontent_secs;
    profile.kantra_eval.secs = kantra_start.elapsed().as_secs_f64();

    let out_path = rgctl_graph::paths::artifact_path(store, "kantra_findings.json");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&findings).context("serialize kantra findings")?;
    fs::write(&out_path, json).with_context(|| format!("write {}", out_path.display()))?;
    Ok(findings)
}

/// Read discovered source files into a cache (parallel).
pub fn preload_discovered_sources(
    repo_root: &Path,
    files: &[PathBuf],
    thread_count: Option<usize>,
) -> SourceCache {
    let paths: HashSet<String> = files
        .iter()
        .filter(|p| is_text_file(p))
        .filter_map(|p| {
            let rel = p
                .strip_prefix(repo_root)
                .unwrap_or(p)
                .to_string_lossy()
                .replace('\\', "/");
            Some(rel)
        })
        .collect();
    rgctl_pipeline::with_pool(thread_count, || {
        paths
            .par_iter()
            .filter_map(|rel| {
                let read_path = if Path::new(rel).is_file() {
                    PathBuf::from(rel)
                } else {
                    repo_root.join(rel)
                };
                let content = fs::read_to_string(&read_path).ok()?;
                Some((rel.clone(), Arc::new(content)))
            })
            .collect()
    })
}

fn is_text_file(path: &Path) -> bool {
    let name = path.to_string_lossy();
    TEXT_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
}

fn build_eval_graph(cold: &ColdMetadataDb, view: &PetGraphView) -> Result<EvalGraph> {
    let mut graph = EvalGraph::default();
    for id in cold.store().all_node_ids() {
        let Some(node) = cold.get_node(id)? else {
            continue;
        };
        if !matches!(
            node.node_type,
            NodeType::Import
                | NodeType::Class
                | NodeType::Interface
                | NodeType::Enum
                | NodeType::Annotation
                | NodeType::Function
                | NodeType::Module
                | NodeType::File
        ) {
            continue;
        }
        graph.nodes.push(node_to_eval(&node));
    }

    let index_to_uuid = &view.index_to_uuid;
    view.topo.for_each_edge(|src, dst, edge_type| {
        let edge_name = match edge_type {
            EdgeType::Extends => "EXTENDS",
            EdgeType::Implements => "IMPLEMENTS",
            EdgeType::AnnotatedWith => "ANNOTATED_WITH",
            _ => return,
        };
        let Some(&from_id) = index_to_uuid.get(src as usize) else {
            return;
        };
        let Some(&to_id) = index_to_uuid.get(dst as usize) else {
            return;
        };
        let Ok(Some(from_node)) = cold.get_node(from_id) else {
            return;
        };
        let Ok(Some(to_node)) = cold.get_node(to_id) else {
            return;
        };
        let file_path = from_node
            .file_path
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();
        let line = from_node.start_line.unwrap_or(1);
        graph.edges.push(EvalEdge {
            edge_type: edge_name.to_string(),
            from_name: from_node.name.to_string(),
            from_qualified: from_node
                .qualified_name
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| from_node.name.to_string()),
            to_name: to_node.name.to_string(),
            to_qualified: to_node
                .qualified_name
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| to_node.name.to_string()),
            file_path,
            line,
        });
    })?;

    Ok(graph)
}

fn node_to_eval(node: &Node) -> EvalNode {
    EvalNode {
        node_type: format!("{:?}", node.node_type),
        name: node.name.to_string(),
        qualified_name: node.qualified_name.as_ref().map(|s| s.to_string()),
        file_path: node.file_path.as_ref().map(|s| s.to_string()),
        start_line: node.start_line,
        labels: node.labels.clone(),
    }
}

/// Validate Kantra CLI flags.
pub fn validate_kantra_flags(
    with_kantra: bool,
    kantra_rules: &Option<String>,
    kantra_catalog: &Option<String>,
) -> Result<()> {
    if !with_kantra {
        return Ok(());
    }
    if kantra_rules.as_ref().is_some_and(|s| !s.is_empty())
        && kantra_catalog.as_ref().is_some_and(|s| !s.is_empty())
    {
        bail!("use only one of --kantra-rules or --kantra-catalog");
    }
    Ok(())
}
