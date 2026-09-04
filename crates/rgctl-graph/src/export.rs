//! Graph export functionality
//!
//! Task 1.6.4: Implement graph export (JSON)

use crate::backend::MemoryBackend;
use crate::schema::{Edge, GRAPH_SCHEMA_VERSION, Node};
use rgctl_error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::io::{BufWriter, Write};

/// Serializable graph snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// rgctl version that created the snapshot
    pub version: String,
    /// Graph schema version (Phase 12.0)
    #[serde(default)]
    pub schema_version: u32,
    /// All nodes
    pub nodes: Vec<Node>,
    /// All edges
    pub edges: Vec<Edge>,
}

/// Export a graph backend to compact JSON.
pub fn export_json(backend: &MemoryBackend) -> Result<String> {
    let mut buf = Vec::new();
    export_json_to(backend, &mut buf)?;
    String::from_utf8(buf).map_err(|e| Error::SerdeError(e.to_string()))
}

/// Stream graph JSON to a writer with a fixed-size buffer (avoids one giant `String`).
pub fn export_json_to<W: Write>(backend: &MemoryBackend, writer: W) -> Result<()> {
    let mut w = BufWriter::with_capacity(8 * 1024, writer);
    writeln!(
        w,
        "{{\n  \"version\": {:?},\n  \"schema_version\": {},",
        env!("CARGO_PKG_VERSION"),
        GRAPH_SCHEMA_VERSION
    )?;
    w.write_all(b"\n  \"nodes\": [\n")?;
    let first = Cell::new(true);
    let mut write_error: Option<Error> = None;
    backend.for_each_node(|node| {
        if write_error.is_some() {
            return;
        }
        let result: Result<()> = (|| {
            if !first.get() {
                w.write_all(b",\n")?;
            }
            first.set(false);
            serde_json::to_writer(&mut w, node).map_err(|e| Error::SerdeError(e.to_string()))?;
            Ok(())
        })();
        if let Err(err) = result {
            write_error = Some(err);
        }
    })?;
    if let Some(err) = write_error {
        return Err(err);
    }
    w.write_all(b"\n  ],\n  \"edges\": [\n")?;
    let edges = backend.all_edges()?;
    for (i, edge) in edges.iter().enumerate() {
        if i > 0 {
            w.write_all(b",\n")?;
        }
        serde_json::to_writer(&mut w, edge).map_err(|e| Error::SerdeError(e.to_string()))?;
    }
    w.write_all(b"\n  ]\n}\n")?;
    w.flush().map_err(|e| Error::SerdeError(e.to_string()))
}

/// Import a graph snapshot from JSON and migrate to the current schema.
pub fn import_json(json: &str) -> Result<GraphSnapshot> {
    let mut snapshot: GraphSnapshot =
        serde_json::from_str(json).map_err(|e| Error::SerdeError(e.to_string()))?;
    snapshot.schema_version = crate::migration::migrate_snapshot(
        snapshot.schema_version,
        &mut snapshot.nodes,
        &mut snapshot.edges,
    )?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::GraphBackend;
    use crate::schema::{EdgeType, NodeType};

    #[test]
    fn test_graph_export_import() {
        let mut backend = MemoryBackend::new();
        let n1 = Node::new(NodeType::Function, "main".to_string());
        let n2 = Node::new(NodeType::File, "main.rs".to_string());
        let id1 = n1.id;
        let id2 = n2.id;
        backend.insert_node(n1).unwrap();
        backend.insert_node(n2).unwrap();
        backend
            .insert_edge(Edge::new(id1, id2, EdgeType::DefinedIn))
            .unwrap();

        let json = export_json(&backend).unwrap();
        let imported = import_json(&json).unwrap();

        assert_eq!(imported.nodes.len(), 2);
        assert_eq!(imported.edges.len(), 1);
    }
}
