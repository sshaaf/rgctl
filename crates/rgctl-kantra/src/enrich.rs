//! Enrich Kantra violations with community, centrality, and blast-radius metrics.

use crate::findings::{KantraFindings, KantraViolation};
use rayon::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

/// Precomputed per-node metrics from discover analysis.
#[derive(Debug, Clone, Default)]
pub struct NodeMetrics {
    pub community_id: Option<usize>,
    pub pagerank: Option<f32>,
    pub blast_radius_score: Option<f64>,
    pub impact_zone_size: Option<usize>,
}

/// Parallel enrichment pass over violations (requires resolved `node_id`s).
pub fn enrich_findings(findings: &mut KantraFindings, metrics: &HashMap<Uuid, NodeMetrics>) {
    findings
        .violations
        .par_iter_mut()
        .for_each(|violation| enrich_violation(violation, metrics));
}

fn enrich_violation(violation: &mut KantraViolation, metrics: &HashMap<Uuid, NodeMetrics>) {
    let Some(node_id) = violation
        .enrichment
        .as_ref()
        .and_then(|e| e.node_id.as_deref())
        .and_then(|s| Uuid::parse_str(s).ok())
    else {
        return;
    };
    let Some(src) = metrics.get(&node_id) else {
        return;
    };
    let enrichment = violation.enrichment.get_or_insert_with(Default::default);
    if enrichment.community_id.is_none() {
        enrichment.community_id = src.community_id;
    }
    if enrichment.pagerank.is_none() {
        enrichment.pagerank = src.pagerank;
    }
    if enrichment.blast_radius_score.is_none() {
        enrichment.blast_radius_score = src.blast_radius_score;
    }
    if enrichment.impact_zone_size.is_none() {
        enrichment.impact_zone_size = src.impact_zone_size;
    }
}
