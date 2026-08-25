//! Hybrid CPG export (P4b) — GraphML / GraphSON over L_repo ∪ optional L_proc.

use crate::cfg_pdg_archive::CfgPdgArchive;
use crate::field_write::FieldWriteIndex;
use rgctl_error::{Error, Result};
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::schema::{EdgeType, NodeType};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Export format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpgExportFormat {
    /// GraphML XML.
    GraphMl,
    /// Neo4j-ish GraphSON (simplified JSON graph).
    GraphSon,
}

/// Scope filter for export.
#[derive(Debug, Clone, Default)]
pub struct CpgExportScope {
    /// If set, keep nodes whose `file_path` contains this substring.
    pub path_contains: Option<String>,
    /// Include CFG NEXT + PDG FLOW from archive when present.
    pub include_l_proc: bool,
    /// Include field-write sites as annotated nodes when index present.
    pub include_field_writes: bool,
}

#[derive(Debug, Serialize)]
struct GraphSonDoc {
    schema_version: u32,
    nodes: Vec<GraphSonNode>,
    edges: Vec<GraphSonEdge>,
}

#[derive(Debug, Serialize)]
struct GraphSonNode {
    id: String,
    labels: Vec<String>,
    properties: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct GraphSonEdge {
    id: String,
    label: String,
    from: String,
    to: String,
    properties: serde_json::Map<String, serde_json::Value>,
}

/// Export a hybrid CPG view to a string.
pub fn export_cpg(
    backend: &MemoryBackend,
    repo_root: &Path,
    format: CpgExportFormat,
    scope: &CpgExportScope,
) -> Result<String> {
    let (nodes, edges) = collect_export_graph(backend, repo_root, scope)?;
    match format {
        CpgExportFormat::GraphSon => {
            let doc = GraphSonDoc {
                schema_version: 1,
                nodes,
                edges,
            };
            serde_json::to_string_pretty(&doc).map_err(|e| Error::SerdeError(e.to_string()))
        }
        CpgExportFormat::GraphMl => Ok(to_graphml(&nodes, &edges)),
    }
}

fn path_ok(scope: &CpgExportScope, file: Option<&str>) -> bool {
    match (&scope.path_contains, file) {
        (None, _) => true,
        (Some(needle), Some(p)) => p.contains(needle.as_str()),
        (Some(_), None) => false,
    }
}

fn collect_export_graph(
    backend: &MemoryBackend,
    repo_root: &Path,
    scope: &CpgExportScope,
) -> Result<(Vec<GraphSonNode>, Vec<GraphSonEdge>)> {
    let all_nodes = backend.all_nodes()?;
    let keep: HashSet<_> = all_nodes
        .iter()
        .filter(|n| {
            matches!(
                n.node_type,
                NodeType::Function
                    | NodeType::Class
                    | NodeType::Struct
                    | NodeType::Interface
                    | NodeType::Variable
            ) && path_ok(scope, n.file_path.as_deref())
        })
        .map(|n| n.id)
        .collect();

    let mut nodes: Vec<GraphSonNode> = all_nodes
        .into_iter()
        .filter(|n| keep.contains(&n.id))
        .map(|n| {
            let mut props = serde_json::Map::new();
            props.insert("name".into(), n.name.as_str().into());
            if let Some(qn) = n.qualified_name {
                props.insert("qualified_name".into(), qn.as_str().into());
            }
            if let Some(f) = n.file_path {
                props.insert("file_path".into(), f.as_str().into());
            }
            if let Some(l) = n.start_line {
                props.insert("start_line".into(), l.into());
            }
            GraphSonNode {
                id: n.id.to_string(),
                labels: vec![format!("{:?}", n.node_type)],
                properties: props,
            }
        })
        .collect();

    let mut edges = Vec::new();
    let mut edge_i = 0usize;
    for e in backend.all_edges()? {
        if !keep.contains(&e.from) || !keep.contains(&e.to) {
            continue;
        }
        if !matches!(
            e.edge_type,
            EdgeType::Calls
                | EdgeType::Contains
                | EdgeType::Extends
                | EdgeType::Implements
                | EdgeType::Uses
                | EdgeType::Modifies
        ) {
            continue;
        }
        edge_i += 1;
        edges.push(GraphSonEdge {
            id: format!("e{edge_i}"),
            label: format!("{:?}", e.edge_type),
            from: e.from.to_string(),
            to: e.to.to_string(),
            properties: serde_json::Map::new(),
        });
    }

    if scope.include_l_proc {
        if let Ok(Some(archive)) = CfgPdgArchive::open_if_exists(repo_root) {
            for record in archive.records.values() {
                if !keep.contains(&record.function_id) {
                    continue;
                }
                // CFG NEXT as self-loop annotation nodes would explode; emit PDG FLOW
                // as edges between synthetic statement ids scoped under the function.
                for dep in &record.pdg.data_deps {
                    if dep.dep_type != crate::pdg::DataDepType::Flow {
                        continue;
                    }
                    let from_line = record
                        .pdg
                        .nodes
                        .get(&dep.from)
                        .map(|n| n.statement.line)
                        .unwrap_or(0);
                    let to_line = record
                        .pdg
                        .nodes
                        .get(&dep.to)
                        .map(|n| n.statement.line)
                        .unwrap_or(0);
                    let from_id = format!("{}:L{}", record.function_id, from_line);
                    let to_id = format!("{}:L{}", record.function_id, to_line);
                    ensure_stmt_node(&mut nodes, &from_id, from_line, &record.function_id);
                    ensure_stmt_node(&mut nodes, &to_id, to_line, &record.function_id);
                    edge_i += 1;
                    let mut props = serde_json::Map::new();
                    props.insert("variable".into(), dep.variable.clone().into());
                    props.insert("loop_carried".into(), dep.loop_carried.into());
                    edges.push(GraphSonEdge {
                        id: format!("e{edge_i}"),
                        label: "DATA_FLOW".into(),
                        from: from_id,
                        to: to_id,
                        properties: props,
                    });
                }
            }
        }
    }

    if scope.include_field_writes {
        if let Ok(Some(index)) = FieldWriteIndex::open_if_exists(repo_root) {
            for (i, w) in index.writes.iter().enumerate() {
                if !path_ok(scope, Some(&w.file)) {
                    continue;
                }
                let id = format!("fw{i}");
                let mut props = serde_json::Map::new();
                props.insert("file".into(), w.file.clone().into());
                props.insert("line".into(), w.line.into());
                props.insert("member".into(), w.member.clone().into());
                props.insert("code".into(), w.code_snippet.clone().into());
                if let Some(t) = &w.receiver_type {
                    props.insert("receiver_type".into(), t.clone().into());
                }
                nodes.push(GraphSonNode {
                    id: id.clone(),
                    labels: vec!["FieldWrite".into()],
                    properties: props,
                });
                edge_i += 1;
                edges.push(GraphSonEdge {
                    id: format!("e{edge_i}"),
                    label: "WRITES_FIELD".into(),
                    from: w.function_id.to_string(),
                    to: id,
                    properties: serde_json::Map::new(),
                });
            }
        }
    }

    Ok((nodes, edges))
}

fn ensure_stmt_node(
    nodes: &mut Vec<GraphSonNode>,
    id: &str,
    line: usize,
    function_id: &uuid::Uuid,
) {
    if nodes.iter().any(|n| n.id == id) {
        return;
    }
    let mut props = serde_json::Map::new();
    props.insert("line".into(), line.into());
    props.insert("function_id".into(), function_id.to_string().into());
    nodes.push(GraphSonNode {
        id: id.to_string(),
        labels: vec!["Statement".into()],
        properties: props,
    });
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn to_graphml(nodes: &[GraphSonNode], edges: &[GraphSonEdge]) -> String {
    let mut out = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<graphml xmlns="http://graphml.graphdrawing.org/xmlns">
  <key id="label" for="node" attr.name="label" attr.type="string"/>
  <key id="props" for="node" attr.name="props" attr.type="string"/>
  <key id="elabel" for="edge" attr.name="label" attr.type="string"/>
  <graph id="cpg" edgedefault="directed">
"#,
    );
    for n in nodes {
        let label = n.labels.first().cloned().unwrap_or_else(|| "Node".into());
        let props = serde_json::to_string(&n.properties).unwrap_or_default();
        out.push_str(&format!(
            "    <node id=\"{}\">\n      <data key=\"label\">{}</data>\n      <data key=\"props\">{}</data>\n    </node>\n",
            xml_escape(&n.id),
            xml_escape(&label),
            xml_escape(&props)
        ));
    }
    for e in edges {
        out.push_str(&format!(
            "    <edge id=\"{}\" source=\"{}\" target=\"{}\">\n      <data key=\"elabel\">{}</data>\n    </edge>\n",
            xml_escape(&e.id),
            xml_escape(&e.from),
            xml_escape(&e.to),
            xml_escape(&e.label)
        ));
    }
    out.push_str("  </graph>\n</graphml>\n");
    out
}
