//! Hybrid graph expansion from semantic search anchors.

use crate::semantic_search::{SemanticEntry, SemanticHit};
use rgctl_error::Result;
use rgctl_graph::backend::{GraphBackend, MemoryBackend};
use rgctl_graph::schema::{EdgeType, Node, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use uuid::Uuid;

/// Which graph expansions to run after semantic retrieval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticExpandMode {
    /// No graph expansion after retrieval.
    #[default]
    None,
    /// Expand CALLS neighbors from semantic anchors.
    Neighbors,
    /// Attach blast-radius summaries for anchors.
    Blast,
    /// Neighbor expansion labeled for GQL-style consumers.
    Gql,
    /// Neighbors + blast + GQL expansions.
    All,
}

/// Config for post-query graph expansion.
#[derive(Debug, Clone)]
pub struct SemanticExpandConfig {
    /// Which expansion modes to run.
    pub mode: SemanticExpandMode,
    /// CALLS hop depth for neighbor / GQL-style expansion.
    pub call_depth: usize,
    /// Max semantic anchors to expand (top hits only).
    pub anchor_limit: usize,
    /// Max related symbols per anchor.
    pub per_anchor_limit: usize,
}

impl Default for SemanticExpandConfig {
    fn default() -> Self {
        Self {
            mode: SemanticExpandMode::None,
            call_depth: 1,
            anchor_limit: 5,
            per_anchor_limit: 20,
        }
    }
}

/// One related symbol from graph expansion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticExpandedNode {
    /// Graph node UUID (string form).
    pub node_id: String,
    /// Short display name.
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Fully qualified name when available.
    pub qualified_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Source file path when available.
    pub file_path: Option<String>,
    /// Relation label (e.g. caller/callee).
    pub relation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Anchor hit this expansion came from.
    pub anchor_node_id: Option<String>,
}

/// Blast-radius summary for one anchor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticBlastSummary {
    /// Anchor node UUID.
    pub anchor_node_id: String,
    /// Anchor display name.
    pub anchor_name: String,
    /// Number of direct callers.
    pub direct_callers: usize,
    /// Impact zone size.
    pub impact_zone: usize,
    /// Blast impact score.
    pub score: f64,
}

/// Combined expansion payload attached to semantic query results.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SemanticExpansion {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// CALLS-neighborhood expansions.
    pub neighbors: Option<Vec<SemanticExpandedNode>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Per-anchor blast summaries.
    pub blast: Option<Vec<SemanticBlastSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Neighbor expansion for GQL-style consumers.
    pub gql: Option<Vec<SemanticExpandedNode>>,
}

/// Expand semantic hits into graph neighborhoods and optional blast summaries.
pub fn expand_semantic_hits(
    backend: &MemoryBackend,
    hits: &[SemanticHit],
    config: &SemanticExpandConfig,
    blast: Option<&dyn BlastSummaryProvider>,
) -> Result<SemanticExpansion> {
    if config.mode == SemanticExpandMode::None || hits.is_empty() {
        return Ok(SemanticExpansion::default());
    }

    let anchors: Vec<&SemanticHit> = hits.iter().take(config.anchor_limit).collect();
    let mut expansion = SemanticExpansion::default();

    if matches!(
        config.mode,
        SemanticExpandMode::Neighbors | SemanticExpandMode::All
    ) {
        expansion.neighbors = Some(expand_call_neighbors(
            backend,
            &anchors,
            config.call_depth,
            config.per_anchor_limit,
        )?);
    }

    if matches!(
        config.mode,
        SemanticExpandMode::Gql | SemanticExpandMode::All
    ) {
        expansion.gql = Some(expand_call_neighbors(
            backend,
            &anchors,
            config.call_depth,
            config.per_anchor_limit,
        )?);
    }

    if matches!(
        config.mode,
        SemanticExpandMode::Blast | SemanticExpandMode::All
    ) {
        if let Some(provider) = blast {
            let mut summaries = Vec::new();
            for hit in &anchors {
                if let Some(summary) = provider.summarize(hit.entry.node_id)? {
                    summaries.push(summary);
                }
            }
            if !summaries.is_empty() {
                expansion.blast = Some(summaries);
            }
        }
    }

    Ok(expansion)
}

/// Trait to supply blast summaries without pulling CLI types into analysis.
pub trait BlastSummaryProvider {
    /// Summarize blast radius for a semantic anchor node.
    fn summarize(&self, anchor_id: Uuid) -> Result<Option<SemanticBlastSummary>>;
}

/// Expand CALLS neighbors up to `depth` hops from each anchor.
pub fn expand_call_neighbors(
    backend: &MemoryBackend,
    anchors: &[&SemanticHit],
    depth: usize,
    per_anchor_limit: usize,
) -> Result<Vec<SemanticExpandedNode>> {
    let depth = depth.max(1);
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for hit in anchors {
        let anchor_id = hit.entry.node_id;
        seen.insert(anchor_id);
        let mut queue = VecDeque::from([(anchor_id, 0usize)]);
        let mut anchor_count = 0usize;

        while let Some((current, hop)) = queue.pop_front() {
            if hop >= depth {
                continue;
            }
            let next_hop = hop + 1;

            for (relation, neighbor_id) in call_edges(backend, current)? {
                if !seen.insert(neighbor_id) {
                    continue;
                }
                if let Some(node) = backend.get_node(neighbor_id)? {
                    if node.node_type != NodeType::Function {
                        continue;
                    }
                    out.push(node_to_expanded(&node, relation, Some(anchor_id)));
                    anchor_count += 1;
                    if anchor_count >= per_anchor_limit {
                        break;
                    }
                    if next_hop < depth {
                        queue.push_back((neighbor_id, next_hop));
                    }
                }
            }
            if anchor_count >= per_anchor_limit {
                break;
            }
        }
    }

    Ok(out)
}

fn call_edges(backend: &MemoryBackend, node_id: Uuid) -> Result<Vec<(String, Uuid)>> {
    let mut edges = Vec::new();
    for edge in backend.get_outgoing_edges(node_id)? {
        if edge.edge_type == EdgeType::Calls {
            edges.push(("calls".into(), edge.to));
        }
    }
    for edge in backend.get_incoming_edges(node_id)? {
        if edge.edge_type == EdgeType::Calls {
            edges.push(("called_by".into(), edge.from));
        }
    }
    Ok(edges)
}

fn node_to_expanded(node: &Node, relation: String, anchor: Option<Uuid>) -> SemanticExpandedNode {
    SemanticExpandedNode {
        node_id: node.id.to_string(),
        name: node.name.to_string(),
        qualified_name: node.qualified_name.as_ref().map(|s| s.to_string()),
        file_path: node.file_path.as_ref().map(|s| s.to_string()),
        relation,
        anchor_node_id: anchor.map(|id| id.to_string()),
    }
}

/// Build blast summary from analysis engine result fields.
pub fn blast_summary_from_result(
    entry: &SemanticEntry,
    direct_callers: usize,
    impact_zone: usize,
    score: f64,
) -> SemanticBlastSummary {
    SemanticBlastSummary {
        anchor_node_id: entry.node_id.to_string(),
        anchor_name: entry
            .qualified_name
            .clone()
            .unwrap_or_else(|| entry.name.clone()),
        direct_callers,
        impact_zone,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_search::SemanticHit;
    use rgctl_graph::backend::GraphBackend;
    use rgctl_graph::schema::{Edge, Node};

    #[test]
    fn expand_call_neighbors_finds_callees_and_callers() {
        let mut backend = MemoryBackend::new();
        let anchor = Node::new(NodeType::Function, "anchor");
        let callee = Node::new(NodeType::Function, "callee");
        let caller = Node::new(NodeType::Function, "caller");
        let anchor_id = anchor.id;
        let callee_id = callee.id;
        let caller_id = caller.id;
        backend.insert_node(anchor).unwrap();
        backend.insert_node(callee.clone()).unwrap();
        backend.insert_node(caller.clone()).unwrap();
        backend
            .insert_edge(Edge::new(anchor_id, callee_id, EdgeType::Calls))
            .unwrap();
        backend
            .insert_edge(Edge::new(caller_id, anchor_id, EdgeType::Calls))
            .unwrap();

        let hit = SemanticHit {
            row: 0,
            distance: 0,
            entry: SemanticEntry {
                node_id: anchor_id,
                name: "anchor".into(),
                qualified_name: None,
                file_path: None,
                code_hash: None,
                node_type: None,
                kind: None,
            },
            fused_score: None,
        };

        let related = expand_call_neighbors(&backend, &[&hit], 1, 10).unwrap();
        let names: HashSet<_> = related.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains("callee"));
        assert!(names.contains("caller"));
    }
}
