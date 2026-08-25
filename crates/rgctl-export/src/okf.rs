//! Open Knowledge Maps (OKF) style JSON export for doc headings.

use rgctl_error::Result;
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::content_store::ContentStore;
use rgctl_graph::schema::{EdgeType, Node, NodeType};
use serde_json::{Value, json};

/// Stats from an OKF JSON export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OkfExportStats {
    /// Entities exported.
    pub entities: usize,
}

/// Export heading modules as OKF-style JSON entities.
pub fn export_okf_json(
    backend: &MemoryBackend,
    content_store: &ContentStore,
) -> Result<(Value, OkfExportStats)> {
    let mut entities: Vec<Value> = Vec::new();

    backend.for_each_node(|node| {
        if node.node_type != NodeType::Module {
            return;
        }
        if node.get_property("kind") != Some("heading") {
            return;
        }
        let qn = node
            .qualified_name
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| node.name.to_string());
        let body = resolve_body(node, content_store);
        entities.push(json!({
            "id": node.id.to_string(),
            "type": "doc_section",
            "title": node.name.to_string(),
            "qualified_name": qn,
            "level": node.get_property("level"),
            "file_path": node.file_path.as_ref().map(|s| s.to_string()),
            "body": body,
            "body_hash": node.get_property("body_hash"),
            "body_ref": node.get_property("body_ref"),
        }));
    })?;

    let mut links: Vec<Value> = Vec::new();
    backend.for_each_edge(|edge| {
        if edge.edge_type != EdgeType::References {
            return;
        }
        let from_qn = backend
            .with_node(edge.from, |n| {
                n.qualified_name.as_ref().map(|s| s.to_string())
            })
            .ok()
            .flatten();
        let to_label = backend
            .with_node(edge.to, |n| link_target_label(n))
            .ok()
            .flatten();
        if let (Some(from), Some(to)) = (from_qn, to_label) {
            links.push(json!({
                "source": from,
                "target": to,
                "type": "references",
            }));
        }
    })?;

    let doc = json!({
        "@context": "https://openknowledgemaps.org/context",
        "schema_version": "okf-export-v1",
        "entities": entities,
        "links": links,
    });
    let count = entities.len();
    Ok((doc, OkfExportStats { entities: count }))
}

fn link_target_label(node: &Node) -> Option<String> {
    if let Some(qn) = node.qualified_name.as_ref() {
        return Some(qn.to_string());
    }
    if node.node_type == NodeType::File {
        return Some(node.name.to_string());
    }
    None
}

fn resolve_body(node: &Node, store: &ContentStore) -> Option<String> {
    if let Some(text) = node.get_property("body_text") {
        return Some(text.to_string());
    }
    if let Some(ref_key) = node.get_property("body_ref") {
        return store.get_str(ref_key).map(|s| s.to_string());
    }
    None
}
