//! Semantic search command.

use crate::command::{SearchArgs, SearchScope};
use crate::error::{Result, ServiceError};
use crate::semantic_json::{SemanticHitJson, build_query_response, hit_from_semantic, query_response_to_json};
use rgctl_analysis::{
    AnalysisResults, CommunityQueryContext, OnnxReloadOptions, SemanticFusionConfig, SemanticIndex,
    query_communities, query_index_with_fusion,
};
use rgctl_graph::CodeGraph;
use rgctl_graph::backend::GraphBackend;
use serde_json::Value;
use std::path::Path;

/// Run semantic query. Missing index is handled by the caller (status JSON).
pub fn run_search(graph: &CodeGraph, repo: &Path, args: &SearchArgs) -> Result<Value> {
    if args.text.trim().is_empty() {
        return Err(ServiceError::InvalidParams("`text` must not be empty".into()));
    }
    let path = SemanticIndex::default_path(repo);
    let index = SemanticIndex::load(&path).map_err(ServiceError::from)?;
    let limit = args.limit.unwrap_or(usize::MAX);
    let reload = OnnxReloadOptions {
        model_path: None,
        tokenizer_path: None,
    };
    let analysis_path = rgctl_graph::paths::artifact_path(repo, "analysis_results.bin");
    let analysis = if analysis_path.is_file() {
        Some(AnalysisResults::load(&analysis_path).map_err(ServiceError::from)?)
    } else {
        None
    };
    let fusion = SemanticFusionConfig {
        enabled: true,
        candidate_pool: limit.max(32),
        keyword_and: false,
        ..SemanticFusionConfig::default()
    };

    if args.scope == SearchScope::Community {
        let analysis = analysis.ok_or_else(|| {
            ServiceError::Failed(
                "community semantic search requires analysis_results.bin (run `rgctl discover`)"
                    .into(),
            )
        })?;
        let backend = graph.backend();
        let ctx = CommunityQueryContext::from_analysis(&analysis, |uuid| {
            backend.get_node(uuid).ok().flatten().map(|n| {
                (
                    n.name.to_string(),
                    n.file_path.as_ref().map(|s| s.to_string()),
                )
            })
        });
        let labels: std::collections::HashMap<_, _> = ctx
            .communities
            .iter()
            .map(|c| (c.id, c.label.clone()))
            .collect();
        let community_hits =
            query_communities(&index, &analysis, &labels, &args.text, limit, &reload)
                .map_err(ServiceError::from)?;
        let hits: Vec<SemanticHitJson> = community_hits
            .into_iter()
            .map(|h| SemanticHitJson {
                node_id: h.community_id.to_string(),
                name: h.label.clone(),
                qualified_name: Some(format!("community:{}", h.community_id)),
                file_path: Some(format!("{} members", h.member_count)),
                distance: h.distance,
                score: h.score,
                fused_score: None,
                ranking: Some("community".into()),
            })
            .collect();
        let response = build_query_response(&args.text, &index.model_id, index.dimensions, hits, None);
        return Ok(query_response_to_json(&response));
    }

    let mut hits = query_index_with_fusion(
        &index,
        &args.text,
        limit,
        &reload,
        &fusion,
        analysis.as_ref(),
        Some(graph.backend()),
        Some(repo),
    )
    .map_err(ServiceError::from)?;
    apply_scope_filter(&mut hits, args.scope);
    if let Some(cap) = args.limit {
        hits.truncate(cap);
    }
    let hit_json: Vec<_> = hits
        .iter()
        .map(|hit| hit_from_semantic(&hit.entry, hit.distance, index.dimensions, Some(hit)))
        .collect();
    let response = build_query_response(
        &args.text,
        &index.model_id,
        index.dimensions,
        hit_json,
        None,
    );
    Ok(query_response_to_json(&response))
}

/// Whether the semantic index file exists.
#[must_use]
pub fn semantic_ready(repo: &Path) -> bool {
    SemanticIndex::default_path(repo).is_file()
}

fn apply_scope_filter(hits: &mut Vec<rgctl_analysis::SemanticHit>, scope: SearchScope) {
    match scope {
        SearchScope::Function => hits.retain(|hit| {
            hit.entry
                .node_type
                .as_deref()
                .is_some_and(|node_type| node_type == "Function")
        }),
        SearchScope::Docs => hits.retain(|hit| {
            hit.entry
                .node_type
                .as_deref()
                .is_some_and(|node_type| node_type == "Module")
                && hit
                    .entry
                    .kind
                    .as_deref()
                    .is_some_and(|kind| kind == "heading" || kind == "code_block")
        }),
        SearchScope::All | SearchScope::Community => {}
    }
}
