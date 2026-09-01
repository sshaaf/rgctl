//! Materialize `VIOLATES` edges from findings into the graph snapshot.

use crate::error::{KantraError, Result};
use crate::findings::KantraFindings;
use crate::index::rule_node_id;
use rgctl_graph::backend::{GraphBackend, MemoryBackend};
use rgctl_graph::schema::{Edge, EdgeType};
use rgctl_graph::snapshot::MmappedGraphSnapshot;
use rgctl_graph::write_columnar_from_backend;
use std::path::Path;
use uuid::Uuid;

/// Append `VIOLATES` edges for resolved violations; replaces prior violation edges.
pub fn materialize_violates_edges(backend: &mut MemoryBackend, findings: &KantraFindings) -> Result<usize> {
    strip_violates_edges(backend)?;
    let mut count = 0usize;
    for violation in &findings.violations {
        let Some(node_id) = violation
            .enrichment
            .as_ref()
            .and_then(|e| e.node_id.as_deref())
            .and_then(|s| Uuid::parse_str(s).ok())
        else {
            continue;
        };
        let rule_id = rule_node_id(&violation.rule_id);
        let mut edge = Edge::new(rule_id, node_id, EdgeType::Violates);
        edge.properties
            .insert("line".into(), violation.line.to_string());
        edge.properties
            .insert("matched_by".into(), violation.matched_by.clone());
        if let Some(sym) = &violation.symbol {
            edge.properties.insert("symbol".into(), sym.clone());
        }
        backend
            .insert_edge(edge)
            .map_err(|e| KantraError::msg(e.to_string()))?;
        count += 1;
    }
    Ok(count)
}

/// Rewrite snapshot with violation edges from findings.
pub fn rewrite_snapshot_with_violations(
    snapshot_path: &Path,
    findings: &KantraFindings,
) -> Result<usize> {
    let snap = MmappedGraphSnapshot::open(snapshot_path)
        .map_err(|e| KantraError::msg(format!("open snapshot: {e}")))?;
    let mut backend = snap
        .hydrate_backend()
        .map_err(|e| KantraError::msg(format!("hydrate snapshot: {e}")))?;
    let count = materialize_violates_edges(&mut backend, findings)?;
    write_columnar_from_backend(&backend, snapshot_path)
        .map_err(|e| KantraError::msg(format!("write snapshot: {e}")))?;
    Ok(count)
}

fn strip_violates_edges(backend: &mut MemoryBackend) -> Result<()> {
    backend
        .strip_edges_by_type(EdgeType::Violates)
        .map_err(|e| KantraError::msg(e.to_string()))?;
    Ok(())
}
