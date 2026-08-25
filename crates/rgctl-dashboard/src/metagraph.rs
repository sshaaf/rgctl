//! Package-level metagraph for Phase 2 community / macro visualization.

use crate::communities::{COMMUNITIES_FILE, CommunitiesPayload, summarize_communities};
use rgctl_analysis::AnalysisResults;
use rgctl_analysis::community::CommunityDetector;
use rgctl_analysis::graph_utils::PetGraphView;
use rgctl_graph::backend::{GraphBackend, MemoryBackend};
use rgctl_graph::schema::{EdgeType, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::Path;
use uuid::Uuid;

pub const METAGRAPH_SCHEMA_VERSION: u32 = 3;
pub const METAGRAPH_FILE: &str = "metagraph.json";
/// Above this raw node count the UI hides per-function nodes (community-only mode).
pub const COMMUNITY_ONLY_THRESHOLD: u64 = 50_000;
pub const MAX_METANODES: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetagraphPayload {
    pub schema_version: u32,
    pub mode: String,
    pub community_only: bool,
    pub threshold_community_only: u64,
    pub source_node_count: u64,
    pub nodes: Vec<Metanode>,
    pub edges: Vec<Metaedge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metanode {
    pub id: u32,
    pub label: String,
    pub size: u32,
    pub functions: u32,
    pub classes: u32,
    pub avg_complexity: f64,
    pub x: f64,
    pub y: f64,
    /// Columnar row indices in `graph_payload.bin` for WASM LOD drill-down.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_indices: Vec<u32>,
    /// Louvain community id (majority vote of member nodes).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub community_id: Option<usize>,
}

/// Metagraph plus community summary written beside the bundle.
#[derive(Debug, Clone)]
pub struct MetagraphExport {
    pub meta: MetagraphPayload,
    pub communities: CommunitiesPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metaedge {
    pub source: u32,
    pub target: u32,
    pub weight: u32,
    pub kind: String,
}

struct PackageBucket {
    label: String,
    functions: u32,
    classes: u32,
    modules: u32,
    files: u32,
    complexity_sum: f64,
    complexity_count: u32,
    member_indices: Vec<u32>,
    community_votes: HashMap<usize, u32>,
}

fn bucket_member_count(bucket: &PackageBucket) -> u32 {
    bucket.functions + bucket.classes + bucket.modules + bucket.files
}

/// Node types included in package metagraph buckets (code + doc structure).
fn is_metagraph_member(node: &rgctl_graph::schema::Node) -> bool {
    match node.node_type {
        NodeType::Function | NodeType::Class | NodeType::File => true,
        NodeType::Module => module_kind_allowed(node.properties.get("kind").map(|s| s.as_str())),
        _ => false,
    }
}

/// Doc `:Module` kinds we bucket; code package modules have no `kind` property.
fn module_kind_allowed(kind: Option<&str>) -> bool {
    match kind {
        None => true,
        Some("heading") | Some("code_block") => true,
        _ => false,
    }
}

fn metagraph_edge_kind(edge_type: EdgeType) -> Option<&'static str> {
    match edge_type {
        EdgeType::Calls => Some("calls"),
        EdgeType::References => Some("references"),
        EdgeType::Uses => Some("uses"),
        _ => None,
    }
}

/// Build package metagraph from indexed graph and write JSON beside the dashboard bundle.
pub fn write_metagraph(
    backend: &MemoryBackend,
    _snapshot_path: &Path,
    out_dir: &Path,
    source_node_count: u64,
    analysis: Option<&AnalysisResults>,
    uuid_to_col: &HashMap<Uuid, u32>,
) -> Result<MetagraphExport, String> {
    let mut uuid_to_pkg: HashMap<Uuid, u32> = HashMap::new();
    let mut packages: HashMap<String, PackageBucket> = HashMap::new();
    let include_member_indices = source_node_count < COMMUNITY_ONLY_THRESHOLD;

    let (modularity, detected_communities) = if analysis.is_some() {
        (
            analysis
                .and_then(|a| a.community.as_ref())
                .map(|c| c.modularity)
                .unwrap_or(0.0),
            None,
        )
    } else {
        let start = std::time::Instant::now();
        let (assignments, modularity) = detect_node_communities(backend)?;
        tracing::info!(
            target: "profile",
            secs = start.elapsed().as_secs_f64(),
            "[profile] write_metagraph community_detect"
        );
        (modularity, Some(assignments))
    };

    let _ = backend.for_each_node(|n| {
        if !is_metagraph_member(n) {
            return;
        }
        let label = package_label(n.file_path.as_deref().unwrap_or(""));
        let bucket = packages
            .entry(label.clone())
            .or_insert_with(|| PackageBucket {
                label: label.clone(),
                functions: 0,
                classes: 0,
                modules: 0,
                files: 0,
                complexity_sum: 0.0,
                complexity_count: 0,
                member_indices: Vec::new(),
                community_votes: HashMap::new(),
            });
        if let Some(cid) = community_id_for_node(analysis, detected_communities.as_ref(), n.id) {
            *bucket.community_votes.entry(cid).or_insert(0) += 1;
        }
        match n.node_type {
            NodeType::Function => bucket.functions += 1,
            NodeType::Class => bucket.classes += 1,
            NodeType::Module => bucket.modules += 1,
            NodeType::File => bucket.files += 1,
            _ => {}
        }
        if let Some(c) = n
            .properties
            .get("cyclomatic")
            .and_then(|v| v.parse::<f64>().ok())
        {
            bucket.complexity_sum += c;
            bucket.complexity_count += 1;
        }
        if include_member_indices {
            if let Some(col_idx) = uuid_to_col.get(&n.id) {
                bucket.member_indices.push(*col_idx);
            }
        }
    });

    if packages.is_empty() {
        packages.insert(
            "default".into(),
            PackageBucket {
                label: "default".into(),
                functions: 0,
                classes: 0,
                modules: 0,
                files: 0,
                complexity_sum: 0.0,
                complexity_count: 0,
                member_indices: Vec::new(),
                community_votes: HashMap::new(),
            },
        );
    }

    let mut ranked: Vec<_> = packages.into_values().collect();
    ranked.sort_by_key(|b| std::cmp::Reverse(bucket_member_count(b)));

    let tail = if ranked.len() > MAX_METANODES {
        ranked.split_off(MAX_METANODES - 1)
    } else {
        vec![]
    };
    let top = ranked;

    let mut metanodes: Vec<Metanode> = Vec::new();
    let mut label_to_id: HashMap<String, u32> = HashMap::new();

    for (idx, bucket) in top.into_iter().enumerate() {
        let id = idx as u32;
        label_to_id.insert(bucket.label.clone(), id);
        metanodes.push(bucket_to_metanode(id, bucket));
    }

    if !tail.is_empty() {
        let id = metanodes.len() as u32;
        let mut merged = PackageBucket {
            label: "(other)".into(),
            functions: 0,
            classes: 0,
            modules: 0,
            files: 0,
            complexity_sum: 0.0,
            complexity_count: 0,
            member_indices: Vec::new(),
            community_votes: HashMap::new(),
        };
        for b in &tail {
            for (cid, votes) in &b.community_votes {
                *merged.community_votes.entry(*cid).or_insert(0) += votes;
            }
        }
        for b in tail {
            merged.functions += b.functions;
            merged.classes += b.classes;
            merged.modules += b.modules;
            merged.files += b.files;
            merged.complexity_sum += b.complexity_sum;
            merged.complexity_count += b.complexity_count;
            merged.member_indices.extend(b.member_indices);
        }
        label_to_id.insert(merged.label.clone(), id);
        metanodes.push(bucket_to_metanode(id, merged));
    }

    let _ = backend.for_each_node(|n| {
        if !is_metagraph_member(n) {
            return;
        }
        let label = package_label(n.file_path.as_deref().unwrap_or(""));
        let pkg_id = *label_to_id
            .get(&label)
            .or_else(|| label_to_id.get("(other)"))
            .unwrap_or(&0);
        uuid_to_pkg.insert(n.id, pkg_id);
    });

    let mut edge_weights: HashMap<(u32, u32, &'static str), u32> = HashMap::new();
    let _ = backend.for_each_edge(|e| {
        let Some(kind) = metagraph_edge_kind(e.edge_type) else {
            return;
        };
        let Some(&from) = uuid_to_pkg.get(&e.from) else {
            return;
        };
        let Some(&to) = uuid_to_pkg.get(&e.to) else {
            return;
        };
        if from == to {
            return;
        }
        *edge_weights.entry((from, to, kind)).or_insert(0) += 1;
    });

    let edges: Vec<Metaedge> = edge_weights
        .into_iter()
        .map(|((source, target, kind), weight)| Metaedge {
            source,
            target,
            weight,
            kind: kind.into(),
        })
        .collect();

    layout_circle(&mut metanodes);

    let community_only = source_node_count >= COMMUNITY_ONLY_THRESHOLD;
    let payload = MetagraphPayload {
        schema_version: METAGRAPH_SCHEMA_VERSION,
        mode: if community_only {
            "community_only".into()
        } else {
            "package_metagraph".into()
        },
        community_only,
        threshold_community_only: COMMUNITY_ONLY_THRESHOLD,
        source_node_count,
        nodes: metanodes,
        edges,
    };

    let label_map = {
        let mut map = analysis
            .and_then(|a| a.community.as_ref())
            .map(|c| c.labels.clone())
            .unwrap_or_default();
        if map.is_empty() {
            if let Some(a) = analysis {
                let ctx = rgctl_analysis::CommunityQueryContext::from_analysis(a, |uuid| {
                    backend.get_node(uuid).ok().flatten().map(|n| {
                        (
                            n.name.to_string(),
                            n.file_path.as_ref().map(|s| s.to_string()),
                        )
                    })
                });
                map = ctx
                    .communities
                    .into_iter()
                    .map(|c| (c.id, c.label))
                    .collect();
            }
        }
        map
    };
    let communities = summarize_communities(modularity, &payload.nodes, &label_map);

    let metagraph_json = {
        let start = std::time::Instant::now();
        let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
        tracing::info!(
            target: "profile",
            file = METAGRAPH_FILE,
            serialize_secs = start.elapsed().as_secs_f64(),
            json_bytes = json.len(),
            member_index_total = payload
                .nodes
                .iter()
                .map(|n| n.member_indices.len())
                .sum::<usize>(),
            "[profile] save_dashboard json serialize"
        );
        json
    };
    let write_start = std::time::Instant::now();
    fs_write(out_dir.join(METAGRAPH_FILE), metagraph_json.as_bytes())?;
    tracing::info!(
        target: "profile",
        file = METAGRAPH_FILE,
        write_secs = write_start.elapsed().as_secs_f64(),
        "[profile] save_dashboard json write"
    );

    let communities_json = {
        let start = std::time::Instant::now();
        let json = serde_json::to_string_pretty(&communities).map_err(|e| e.to_string())?;
        tracing::info!(
            target: "profile",
            file = COMMUNITIES_FILE,
            serialize_secs = start.elapsed().as_secs_f64(),
            json_bytes = json.len(),
            "[profile] save_dashboard json serialize"
        );
        json
    };
    let write_start = std::time::Instant::now();
    fs_write(out_dir.join(COMMUNITIES_FILE), communities_json.as_bytes())?;
    tracing::info!(
        target: "profile",
        file = COMMUNITIES_FILE,
        write_secs = write_start.elapsed().as_secs_f64(),
        "[profile] save_dashboard json write"
    );

    Ok(MetagraphExport {
        meta: payload,
        communities,
    })
}

fn community_id_for_node(
    analysis: Option<&AnalysisResults>,
    detected: Option<&HashMap<Uuid, usize>>,
    node_id: Uuid,
) -> Option<usize> {
    if let Some(ar) = analysis {
        let compact = ar.get_compact_id(node_id)?;
        return ar.community.as_ref()?.get(compact);
    }
    detected.and_then(|m| m.get(&node_id).copied())
}

fn detect_node_communities(backend: &MemoryBackend) -> Result<(HashMap<Uuid, usize>, f64), String> {
    let view = PetGraphView::from_backend(backend).map_err(|e| e.to_string())?;
    let result = CommunityDetector::new()
        .detect_with_view(&view)
        .map_err(|e| e.to_string())?;
    Ok((result.assignments, result.modularity))
}

fn bucket_to_metanode(id: u32, bucket: PackageBucket) -> Metanode {
    let size = bucket_member_count(&bucket);
    let avg_complexity = if bucket.complexity_count > 0 {
        bucket.complexity_sum / bucket.complexity_count as f64
    } else {
        0.0
    };
    let community_id = bucket
        .community_votes
        .into_iter()
        .max_by_key(|(_, votes)| *votes)
        .map(|(cid, _)| cid);
    Metanode {
        id,
        label: bucket.label,
        size,
        functions: bucket.functions,
        classes: bucket.classes,
        avg_complexity,
        x: 0.0,
        y: 0.0,
        member_indices: bucket.member_indices,
        community_id,
    }
}

fn layout_circle(nodes: &mut [Metanode]) {
    let n = nodes.len().max(1) as f64;
    let radius = (n * 12.0).sqrt().max(40.0);
    for (i, node) in nodes.iter_mut().enumerate() {
        let angle = 2.0 * PI * i as f64 / n;
        node.x = angle.cos() * radius;
        node.y = angle.sin() * radius;
    }
}

/// Derive a stable package / module label from a source file path.
pub fn package_label(file_path: &str) -> String {
    let path = file_path.replace('\\', "/");
    if let Some(idx) = path.find("/java/") {
        let after = &path[idx + 6..];
        if let Some(parent) = std::path::Path::new(after).parent() {
            let pkg = parent.to_string_lossy().replace('/', ".");
            if !pkg.is_empty() {
                return pkg;
            }
        }
    }
    if let Some(idx) = path.find("/src/") {
        let after = &path[idx + 5..];
        if let Some(parent) = std::path::Path::new(after).parent() {
            let pkg = parent.to_string_lossy().replace('/', ".");
            if !pkg.is_empty() {
                return pkg;
            }
        }
    }
    std::path::Path::new(&path)
        .parent()
        .map(|p| p.to_string_lossy().replace('/', "."))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "root".into())
}

fn fs_write(path: std::path::PathBuf, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgctl_graph::schema::{Edge, Node};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn doc_heading(name: &str, file: &str) -> Node {
        Node::new(NodeType::Module, name)
            .with_file_path(file)
            .with_property("kind".to_string(), "heading".to_string())
    }

    #[test]
    fn markdown_only_graph_produces_communities_and_metanodes() {
        let mut backend = MemoryBackend::new();

        let file_a = Node::new(NodeType::File, "content/en/docs/a.md")
            .with_file_path("content/en/docs/a.md");
        let file_b = Node::new(NodeType::File, "content/en/blog/b.md")
            .with_file_path("content/en/blog/b.md");
        let h_a = doc_heading("Alpha", "content/en/docs/a.md");
        let h_b = doc_heading("Beta", "content/en/blog/b.md");
        let h_c = doc_heading("Gamma", "content/en/blog/b.md");

        let id_file_a = file_a.id;
        let id_file_b = file_b.id;
        let id_h_a = h_a.id;
        let id_h_b = h_b.id;
        let id_h_c = h_c.id;

        backend.insert_node(file_a).unwrap();
        backend.insert_node(file_b).unwrap();
        backend.insert_node(h_a).unwrap();
        backend.insert_node(h_b).unwrap();
        backend.insert_node(h_c).unwrap();

        backend
            .insert_edge(Edge::new(id_h_a, id_h_b, EdgeType::References))
            .unwrap();
        backend
            .insert_edge(Edge::new(id_h_b, id_h_c, EdgeType::References))
            .unwrap();
        backend
            .insert_edge(Edge::new(id_h_a, id_file_b, EdgeType::References))
            .unwrap();
        backend
            .insert_edge(Edge::new(id_file_a, id_h_a, EdgeType::Contains))
            .unwrap();
        backend
            .insert_edge(Edge::new(id_file_b, id_h_b, EdgeType::Contains))
            .unwrap();

        let dir = tempdir().unwrap();
        let export = write_metagraph(
            &backend,
            Path::new("graph.snapshot.bin"),
            dir.path(),
            5,
            None,
            &HashMap::new(),
        )
        .unwrap();

        assert!(
            export.meta.nodes.iter().any(|n| n.size > 0),
            "metanodes must count doc modules/files"
        );
        assert!(
            !export.meta.nodes.iter().all(|n| n.label == "default"),
            "expected directory buckets, not placeholder default only"
        );
        assert!(
            export.meta.edges.iter().any(|e| e.kind == "references"),
            "metaedges must include REFERENCES for doc graphs"
        );
        assert!(
            !export.communities.communities.is_empty(),
            "communities.json must list communities for markdown REFERENCES graph"
        );
    }

    #[test]
    fn java_package_label() {
        assert_eq!(
            package_label("src/main/java/com/example/foo/Bar.java"),
            "com.example.foo"
        );
    }

    #[test]
    fn rust_module_label() {
        assert_eq!(
            package_label("src/graph/detection/mod.rs"),
            "src.graph.detection"
        );
    }

    #[test]
    fn markdown_heading_modules_are_metagraph_members() {
        let heading = rgctl_graph::schema::Node::new(NodeType::Module, "Intro")
            .with_property("kind".to_string(), "heading".to_string());
        let link = rgctl_graph::schema::Node::new(NodeType::Import, "link");
        assert!(is_metagraph_member(&heading));
        assert!(!is_metagraph_member(&link));
    }

    #[test]
    fn references_edges_map_to_metagraph_kind() {
        assert_eq!(
            metagraph_edge_kind(EdgeType::References),
            Some("references")
        );
        assert_eq!(metagraph_edge_kind(EdgeType::Calls), Some("calls"));
        assert_eq!(metagraph_edge_kind(EdgeType::Contains), None);
    }
}
