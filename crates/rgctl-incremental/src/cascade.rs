//! Reverse-dependency expansion for incremental graph updates.

use rgctl_error::Result;
use rgctl_graph::columnar_snapshot::ColumnarGraphMmap;
use rgctl_graph::normalize_path_str;
use rgctl_graph::schema::EdgeType;
use rgctl_graph::stable_key::{node_row_ref, node_scope_path_at};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn path_matches_scope(path: &str, scope: &HashSet<String>) -> bool {
    let norm = normalize_path_str(path);
    scope.contains(&norm)
        || scope.iter().any(|p| norm.ends_with(p.as_str()) || p.ends_with(&norm))
}

fn node_ids_for_files(
    file_to_ids: &HashMap<String, HashSet<Uuid>>,
    files: &HashSet<String>,
) -> HashSet<Uuid> {
    let mut ids = HashSet::new();
    for (file, node_ids) in file_to_ids {
        if path_matches_scope(file, files) {
            ids.extend(node_ids);
        }
    }
    ids
}

/// Collect declaring files for nodes with incoming `Calls` edges into `seed_files` (1 hop).
pub fn incoming_callers_files(
    col: &ColumnarGraphMmap,
    seed_files: &[String],
) -> Result<Vec<String>> {
    incoming_callers_files_depth(col, seed_files, 1)
}

/// Expand `seed_files` by up to `depth` hops of reverse call dependencies.
pub fn incoming_callers_files_depth(
    col: &ColumnarGraphMmap,
    seed_files: &[String],
    depth: usize,
) -> Result<Vec<String>> {
    if depth == 0 || seed_files.is_empty() {
        return Ok(Vec::new());
    }

    let seeds: HashSet<String> = seed_files.iter().map(|p| normalize_path_str(p)).collect();
    let mut id_to_path: HashMap<Uuid, String> = HashMap::new();
    let mut file_to_ids: HashMap<String, HashSet<Uuid>> = HashMap::new();

    for idx in 0..col.node_count() {
        let id = node_row_ref(col, idx)?.id;
        if let Some(path) = node_scope_path_at(col, idx)? {
            let norm = normalize_path_str(path);
            id_to_path.insert(id, norm.clone());
            file_to_ids.entry(norm).or_default().insert(id);
        }
    }

    let mut expanded: HashSet<String> = HashSet::new();
    let mut visited_files = seeds.clone();
    let mut target_nodes = node_ids_for_files(&file_to_ids, &seeds);

    for _ in 0..depth {
        let mut caller_files: HashSet<String> = HashSet::new();
        col.for_each_edge(|from, to, edge_type| {
            if edge_type == EdgeType::Calls && target_nodes.contains(&to) {
                if let Some(path) = id_to_path.get(&from) {
                    if visited_files.insert(path.clone()) {
                        caller_files.insert(path.clone());
                    }
                }
            }
            Ok(())
        })?;

        if caller_files.is_empty() {
            break;
        }
        expanded.extend(caller_files.iter().cloned());
        target_nodes = node_ids_for_files(&file_to_ids, &caller_files);
    }

    Ok(expanded.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rgctl_graph::schema::{Edge, Node, NodeType};
    use rgctl_graph::{MmappedGraphSnapshot, write_columnar_from_nodes_edges};
    use tempfile::TempDir;

    fn open_fixture(nodes: Vec<Node>, edges: Vec<Edge>) -> (TempDir, MmappedGraphSnapshot) {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("snap.bin");
        write_columnar_from_nodes_edges(nodes, edges, &path).unwrap();
        let store = MmappedGraphSnapshot::open(&path).unwrap();
        (tmp, store)
    }

    #[test]
    fn incoming_callers_files_one_hop() {
        let main_fn = Node::new(NodeType::Function, "main").with_file_path("main.rs");
        let login = Node::new(NodeType::Function, "login").with_file_path("auth.rs");
        let call = Edge::new(main_fn.id, login.id, EdgeType::Calls);
        let (_tmp, store) = open_fixture(vec![main_fn, login], vec![call]);
        let col = store.columnar().expect("columnar");

        let callers = incoming_callers_files(col, &["auth.rs".into()]).unwrap();
        assert_eq!(callers, vec!["main.rs".to_string()]);
    }

    #[test]
    fn incoming_callers_files_depth_zero_is_empty() {
        let main_fn = Node::new(NodeType::Function, "main").with_file_path("main.rs");
        let login = Node::new(NodeType::Function, "login").with_file_path("auth.rs");
        let call = Edge::new(main_fn.id, login.id, EdgeType::Calls);
        let (_tmp, store) = open_fixture(vec![main_fn, login], vec![call]);
        let col = store.columnar().expect("columnar");

        let callers = incoming_callers_files_depth(col, &["auth.rs".into()], 0).unwrap();
        assert!(callers.is_empty());
    }
}
