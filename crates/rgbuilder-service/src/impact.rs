//! Blast-radius (impact) command.

use crate::blast_json::{
    NodeLookup, build_from_engine_result, response_to_json, skipped_gatekeeping,
};
use crate::command::ImpactArgs;
use crate::error::{Result, ServiceError};
use rgbuilder_analysis::{
    BlastRadiusEngine, MacroCallIndex, MacroCallLookupDb, PetGraphView, candidates_from_backend,
    filter_impact_by_caller_depth, impact_score_from_counts, parse_fqn_symbol, resolve_symbol_uuid,
    try_load_engine, try_parse_symbol_uuid,
};
use rgbuilder_graph::CodeGraph;
use rgbuilder_graph::backend::GraphBackend;
use serde_json::Value;
use std::path::Path;
use uuid::Uuid;

/// Run blast-radius on a loaded graph.
pub fn run_impact(graph: &CodeGraph, repo: &Path, args: &ImpactArgs) -> Result<Value> {
    if args.symbol.trim().is_empty() {
        return Err(ServiceError::InvalidParams("`symbol` is required".into()));
    }
    let parsed = parse_fqn_symbol(&args.symbol, args.class.clone(), args.file.clone());
    let backend = graph.backend();
    let digest = crate::session::graph_digest_for(repo);
    let (id, _) = resolve_target(backend, repo, &parsed)?;
    let result = resolve_blast_result(backend, repo, id, digest.as_deref())?;
    let view = PetGraphView::from_backend(backend).map_err(ServiceError::from)?;
    let function_impact = BlastRadiusEngine::filter_function_impact(backend, &result.impact_zone_ids)
        .map_err(ServiceError::from)?;
    let max_depth = args.depth.unwrap_or(usize::MAX);
    let impact_ids = if max_depth == usize::MAX {
        function_impact
    } else {
        filter_impact_by_caller_depth(&view, id, &function_impact, max_depth)
    };
    let score = if max_depth == usize::MAX {
        result.score
    } else {
        impact_score_from_counts(result.direct_caller_ids.len(), impact_ids.len())
    };
    let caller_depth_limit = if max_depth == usize::MAX {
        None
    } else {
        Some(max_depth)
    };
    let lookup = NodeLookup::Backend(backend);
    let response = build_from_engine_result(
        &args.symbol,
        parsed.class_filter.clone(),
        &result,
        &result.direct_caller_ids,
        &impact_ids,
        score,
        caller_depth_limit,
        lookup,
        skipped_gatekeeping(),
    );
    Ok(response_to_json(&response))
}

fn resolve_target(
    backend: &rgbuilder_graph::backend::MemoryBackend,
    repo: &Path,
    parsed: &rgbuilder_analysis::ParsedSymbol,
) -> Result<(Uuid, String)> {
    if let Some(id) = try_parse_symbol_uuid(&parsed.target_name) {
        let name = backend
            .get_node(id)
            .ok()
            .flatten()
            .map(|n| n.name.to_string())
            .unwrap_or_else(|| parsed.target_name.clone());
        return Ok((id, name));
    }
    let lookup_db = MacroCallLookupDb::default_path(repo);
    if MacroCallLookupDb::is_valid_for_repo(&lookup_db, repo).unwrap_or(false) {
        let candidates = MacroCallLookupDb::get_candidates(&lookup_db, &parsed.target_name)
            .map_err(ServiceError::from)?;
        if !candidates.is_empty() {
            let id = resolve_symbol_uuid(&candidates, parsed).map_err(ServiceError::from)?;
            return Ok((id, parsed.target_name.clone()));
        }
    }
    let index_path = MacroCallIndex::default_path(repo);
    if let Ok(Some(index)) = MacroCallIndex::load(&index_path) {
        if index.is_valid_for_repo(repo).unwrap_or(false) {
            let candidates = index.get_candidates(&parsed.target_name);
            if !candidates.is_empty() {
                let id = resolve_symbol_uuid(&candidates, parsed).map_err(ServiceError::from)?;
                return Ok((id, parsed.target_name.clone()));
            }
        }
    }
    let candidates = candidates_from_backend(backend, &parsed.target_name)?;
    if candidates.is_empty() {
        return Err(ServiceError::Failed(format!(
            "Node not found: {}",
            parsed.target_name
        )));
    }
    let id = resolve_symbol_uuid(&candidates, parsed).map_err(ServiceError::from)?;
    Ok((id, parsed.target_name.clone()))
}

fn resolve_blast_result(
    backend: &rgbuilder_graph::backend::MemoryBackend,
    repo: &Path,
    symbol_id: Uuid,
    graph_digest: Option<&str>,
) -> Result<rgbuilder_analysis::BlastRadiusResult> {
    if let Some(digest) = graph_digest {
        if let Some(engine) = try_load_engine(repo, digest).map_err(ServiceError::from)? {
            return engine.analyze(symbol_id).map_err(ServiceError::from);
        }
    }
    let engine = BlastRadiusEngine::build(backend).map_err(ServiceError::from)?;
    engine.analyze(symbol_id).map_err(ServiceError::from)
}
