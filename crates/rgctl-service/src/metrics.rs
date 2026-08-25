//! PageRank / betweenness / community metrics.

use crate::command::MetricsArgs;
use crate::error::{Result, ServiceError};
use crate::metrics_json::{
    MetricsCommunitiesSection, MetricsPagerankSection, build_metrics_response, metrics_response_to_json,
};
use rgctl_analysis::{
    BetweennessCentrality, CommunityDetector, FastPageRank, PetGraphView,
    community_edge_types_for_backend, default_behavioral_edges,
};
use rgctl_graph::CodeGraph;
use serde_json::{Value, json};

/// Compute requested metric sections.
pub fn run_metrics(graph: &CodeGraph, args: &MetricsArgs) -> Result<Value> {
    if !args.pagerank && !args.betweenness && !args.communities {
        return Err(ServiceError::InvalidParams(
            "at least one of pagerank, betweenness, communities is required".into(),
        ));
    }
    let view = PetGraphView::from_backend(graph.backend()).map_err(ServiceError::from)?;
    let iterations = 20;
    let allowed = default_behavioral_edges();
    let mut pagerank = None;
    let mut betweenness = None;
    let mut communities = None;

    if args.pagerank {
        let engine = FastPageRank::new(iterations, 0.85);
        let (scores, stats) = engine.compute(&view, allowed);
        let mut top: Vec<_> = scores
            .iter()
            .filter(|(_, score)| **score > 0.0)
            .map(|(id, score)| (*id, *score))
            .collect();
        if top.is_empty() {
            top = scores.iter().map(|(id, score)| (*id, *score)).collect();
        }
        top.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        top.truncate(20);
        let top = top
            .iter()
            .map(|(id, score)| json!({ "node": id.to_string(), "pagerank": score }))
            .collect();
        pagerank = Some(MetricsPagerankSection {
            top,
            converged: stats.converged,
            iterations: stats.iterations_run,
            max_delta: stats.max_delta,
        });
    }

    if args.betweenness {
        let bc = BetweennessCentrality::compute_unbounded(&view, allowed);
        let mut top: Vec<_> = bc.iter().map(|(id, score)| (id, *score)).collect();
        top.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        top.truncate(20);
        betweenness = Some(
            top.iter()
                .map(|(id, s)| json!({ "node": id.to_string(), "score": s }))
                .collect(),
        );
    }

    if args.communities {
        let detector = CommunityDetector::new();
        let allowed_comm = community_edge_types_for_backend(graph.backend());
        let result = detector
            .detect_with_view_filtered(&view, &allowed_comm)
            .map_err(ServiceError::from)?;
        communities = Some(MetricsCommunitiesSection {
            count: result.communities.len(),
            modularity: result.modularity,
            assignments: result.assignments.len(),
        });
    }

    Ok(metrics_response_to_json(&build_metrics_response(
        pagerank,
        betweenness,
        communities,
    )))
}
