//! Read-only structural diff between two columnar graph snapshots.

use crate::schema::EdgeType;
use crate::snapshot::SnapshotNodeStore;
use crate::stable_key::{NodeRowRef, StableNodeKey, node_row_ref, stable_key_from_row};
use rayon::prelude::*;
use rgctl_error::{Error, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

const NODE_INDEX_SHARDS: usize = 16;

/// Pair of base/head snapshots opened for comparison.
pub struct SnapshotPair {
    /// Base (pre-change) snapshot store.
    pub base: Arc<SnapshotNodeStore>,
    /// Head (post-change) snapshot store.
    pub head: Arc<SnapshotNodeStore>,
}

impl SnapshotPair {
    /// Open base and head snapshot files in parallel.
    pub fn open(base_path: &Path, head_path: &Path) -> Result<Self> {
        let (base, head) = rayon::join(
            || SnapshotNodeStore::open(base_path),
            || SnapshotNodeStore::open(head_path),
        );
        Ok(Self {
            base: Arc::new(base?),
            head: Arc::new(head?),
        })
    }

    /// True when topology digests match (fast-path empty diff).
    pub fn digest_equal(&self) -> Result<bool> {
        Ok(self.base.content_digest()? == self.head.content_digest()?)
    }
}

/// Node change classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeDeltaKind {
    /// Present on head only.
    Added,
    /// Present on base only.
    Removed,
    /// Stable key matches but row metadata differs.
    Changed,
}

/// Edge change classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeDeltaKind {
    /// Present on head only.
    Added,
    /// Present on base only.
    Removed,
}

/// Node delta event emitted to a [`DiffSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeDeltaEvent {
    /// How the node changed between snapshots.
    pub kind: NodeDeltaKind,
    /// Cross-snapshot stable identity.
    pub key: StableNodeKey,
    /// Base row metadata when present.
    pub base: Option<NodeRowRef>,
    /// Head row metadata when present.
    pub head: Option<NodeRowRef>,
}

/// Edge delta event emitted to a [`DiffSink`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeDeltaEvent {
    /// How the edge changed between snapshots.
    pub kind: EdgeDeltaKind,
    /// Stable key of the source node.
    pub from: StableNodeKey,
    /// Stable key of the target node.
    pub to: StableNodeKey,
    /// Edge relation type.
    pub edge_type: EdgeType,
}

/// Aggregate counts from a snapshot diff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct DiffStats {
    /// Nodes added on head.
    pub nodes_added: usize,
    /// Nodes removed from base.
    pub nodes_removed: usize,
    /// Nodes with matching stable keys but changed metadata.
    pub nodes_changed: usize,
    /// Edges added on head.
    pub edges_added: usize,
    /// Edges removed from base.
    pub edges_removed: usize,
}

/// Callback sink for streaming snapshot diffs.
pub trait DiffSink {
    /// Handle one node delta.
    fn on_node(&mut self, event: NodeDeltaEvent) -> Result<()>;
    /// Handle one edge delta.
    fn on_edge(&mut self, event: EdgeDeltaEvent) -> Result<()>;
}

/// Collect diff events into vectors (tests and debugging).
#[derive(Clone, Debug, Default)]
pub struct VecDiffSink {
    /// Collected node deltas.
    pub nodes: Vec<NodeDeltaEvent>,
    /// Collected edge deltas.
    pub edges: Vec<EdgeDeltaEvent>,
}

impl DiffSink for VecDiffSink {
    fn on_node(&mut self, event: NodeDeltaEvent) -> Result<()> {
        self.nodes.push(event);
        Ok(())
    }

    fn on_edge(&mut self, event: EdgeDeltaEvent) -> Result<()> {
        self.edges.push(event);
        Ok(())
    }
}

/// No-op sink for benchmarks.
pub struct NoopDiffSink;

impl DiffSink for NoopDiffSink {
    fn on_node(&mut self, _event: NodeDeltaEvent) -> Result<()> {
        Ok(())
    }

    fn on_edge(&mut self, _event: EdgeDeltaEvent) -> Result<()> {
        Ok(())
    }
}

/// Structural diff between two snapshots (columnar v2 required).
pub fn diff_snapshots(
    base: &SnapshotNodeStore,
    head: &SnapshotNodeStore,
    sink: &mut dyn DiffSink,
) -> Result<DiffStats> {
    if base.content_digest()? == head.content_digest()? {
        return Ok(DiffStats::default());
    }

    let base_col = require_columnar(base)?;
    let head_col = require_columnar(head)?;

    let head_index = build_node_index_parallel(head_col)?;
    let base_index = build_node_index_sequential(base_col)?;

    let mut stats = DiffStats::default();
    diff_nodes(&base_index, &head_index, sink, &mut stats)?;
    diff_edges(base_col, head_col, &base_index, &head_index, sink, &mut stats)?;
    Ok(stats)
}

fn require_columnar(store: &SnapshotNodeStore) -> Result<&crate::columnar_snapshot::ColumnarGraphMmap> {
    store
        .columnar()
        .ok_or_else(|| Error::SerdeError("snapshot diff requires columnar v2 snapshots".into()))
}

fn build_node_index_sequential(
    col: &crate::columnar_snapshot::ColumnarGraphMmap,
) -> Result<HashMap<StableNodeKey, NodeRowRef>> {
    let mut map = HashMap::with_capacity(col.node_count());
    for idx in 0..col.node_count() {
        let key = stable_key_from_row(col, idx)?;
        let row_ref = node_row_ref(col, idx)?;
        map.insert(key, row_ref);
    }
    Ok(map)
}

fn build_node_index_parallel(
    col: &crate::columnar_snapshot::ColumnarGraphMmap,
) -> Result<HashMap<StableNodeKey, NodeRowRef>> {
    let count = col.node_count();
    let shard_maps: Vec<HashMap<StableNodeKey, NodeRowRef>> = (0..NODE_INDEX_SHARDS)
        .into_par_iter()
        .map(|shard| {
            let mut map = HashMap::new();
            for idx in (0..count).filter(|i| i % NODE_INDEX_SHARDS == shard) {
                if let Ok(key) = stable_key_from_row(col, idx) {
                    if let Ok(row_ref) = node_row_ref(col, idx) {
                        map.insert(key, row_ref);
                    }
                }
            }
            map
        })
        .collect();

    let mut merged = HashMap::with_capacity(count);
    for shard in shard_maps {
        merged.extend(shard);
    }
    Ok(merged)
}

fn diff_nodes(
    base_index: &HashMap<StableNodeKey, NodeRowRef>,
    head_index: &HashMap<StableNodeKey, NodeRowRef>,
    sink: &mut dyn DiffSink,
    stats: &mut DiffStats,
) -> Result<()> {
    for (key, base_ref) in base_index {
        match head_index.get(key) {
            None => {
                stats.nodes_removed += 1;
                sink.on_node(NodeDeltaEvent {
                    kind: NodeDeltaKind::Removed,
                    key: *key,
                    base: Some(*base_ref),
                    head: None,
                })?;
            }
            Some(head_ref) if head_ref.extension_digest != base_ref.extension_digest => {
                stats.nodes_changed += 1;
                sink.on_node(NodeDeltaEvent {
                    kind: NodeDeltaKind::Changed,
                    key: *key,
                    base: Some(*base_ref),
                    head: Some(*head_ref),
                })?;
            }
            Some(_) => {}
        }
    }

    for (key, head_ref) in head_index {
        if !base_index.contains_key(key) {
            stats.nodes_added += 1;
            sink.on_node(NodeDeltaEvent {
                kind: NodeDeltaKind::Added,
                key: *key,
                base: None,
                head: Some(*head_ref),
            })?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct EdgeStableKey {
    from: StableNodeKey,
    to: StableNodeKey,
    edge_type: EdgeType,
}

impl PartialOrd for EdgeStableKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EdgeStableKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.from
            .cmp(&other.from)
            .then_with(|| self.to.cmp(&other.to))
            .then_with(|| {
                crate::csr::edge_type_to_u8(self.edge_type)
                    .cmp(&crate::csr::edge_type_to_u8(other.edge_type))
            })
    }
}

fn diff_edges(
    base_col: &crate::columnar_snapshot::ColumnarGraphMmap,
    head_col: &crate::columnar_snapshot::ColumnarGraphMmap,
    base_index: &HashMap<StableNodeKey, NodeRowRef>,
    head_index: &HashMap<StableNodeKey, NodeRowRef>,
    sink: &mut dyn DiffSink,
    stats: &mut DiffStats,
) -> Result<()> {
    let base_uuid_to_stable = uuid_to_stable_map(base_index);
    let head_uuid_to_stable = uuid_to_stable_map(head_index);

    let mut base_edges = Vec::with_capacity(base_col.edge_count());
    for idx in 0..base_col.edge_count() {
        let (from, to, edge_type) = base_col.edge_at(idx)?;
        let Some(from_key) = base_uuid_to_stable.get(&from) else {
            continue;
        };
        let Some(to_key) = base_uuid_to_stable.get(&to) else {
            continue;
        };
        base_edges.push(EdgeStableKey {
            from: *from_key,
            to: *to_key,
            edge_type,
        });
    }
    base_edges.sort_unstable();

    let mut head_edges = Vec::with_capacity(head_col.edge_count());
    for idx in 0..head_col.edge_count() {
        let (from, to, edge_type) = head_col.edge_at(idx)?;
        let Some(from_key) = head_uuid_to_stable.get(&from) else {
            continue;
        };
        let Some(to_key) = head_uuid_to_stable.get(&to) else {
            continue;
        };
        head_edges.push(EdgeStableKey {
            from: *from_key,
            to: *to_key,
            edge_type,
        });
    }
    head_edges.sort_unstable();

    let mut base_i = 0usize;
    let mut head_i = 0usize;
    while base_i < base_edges.len() && head_i < head_edges.len() {
        let base_key = base_edges[base_i];
        let head_key = head_edges[head_i];
        match head_key.cmp(&base_key) {
            std::cmp::Ordering::Less => {
                emit_edge_added(head_key, sink, stats)?;
                head_i += 1;
            }
            std::cmp::Ordering::Greater => {
                emit_edge_removed(base_key, sink, stats)?;
                base_i += 1;
            }
            std::cmp::Ordering::Equal => {
                base_i += 1;
                head_i += 1;
            }
        }
    }
    while base_i < base_edges.len() {
        emit_edge_removed(base_edges[base_i], sink, stats)?;
        base_i += 1;
    }
    while head_i < head_edges.len() {
        emit_edge_added(head_edges[head_i], sink, stats)?;
        head_i += 1;
    }
    Ok(())
}

fn uuid_to_stable_map(index: &HashMap<StableNodeKey, NodeRowRef>) -> HashMap<Uuid, StableNodeKey> {
    index
        .iter()
        .map(|(key, row)| (row.id, *key))
        .collect()
}

fn emit_edge_removed(
    key: EdgeStableKey,
    sink: &mut dyn DiffSink,
    stats: &mut DiffStats,
) -> Result<()> {
    stats.edges_removed += 1;
    sink.on_edge(EdgeDeltaEvent {
        kind: EdgeDeltaKind::Removed,
        from: key.from,
        to: key.to,
        edge_type: key.edge_type,
    })
}

fn emit_edge_added(
    key: EdgeStableKey,
    sink: &mut dyn DiffSink,
    stats: &mut DiffStats,
) -> Result<()> {
    stats.edges_added += 1;
    sink.on_edge(EdgeDeltaEvent {
        kind: EdgeDeltaKind::Added,
        from: key.from,
        to: key.to,
        edge_type: key.edge_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Edge, Node, NodeType};
    use crate::write_columnar_from_nodes_edges;
    use tempfile::TempDir;

    fn write_snap(
        dir: &Path,
        name: &str,
        nodes: Vec<Node>,
        edges: Vec<Edge>,
    ) -> std::path::PathBuf {
        let path = dir.join(name);
        write_columnar_from_nodes_edges(nodes, edges, &path).unwrap();
        path
    }

    #[test]
    fn snapshot_pair_open_and_digest_fast_path() {
        let tmp = TempDir::new().unwrap();
        let node = Node::new(NodeType::Function, "a").with_file_path("f.rs");
        let path = write_snap(&tmp.path(), "snap.bin", vec![node], vec![]);
        let pair = SnapshotPair::open(&path, &path).unwrap();
        assert!(pair.digest_equal().unwrap());
        let mut sink = VecDiffSink::default();
        let stats = diff_snapshots(&pair.base, &pair.head, &mut sink).unwrap();
        assert_eq!(stats, DiffStats::default());
        assert!(sink.nodes.is_empty());
        assert!(sink.edges.is_empty());
    }

    #[test]
    fn diff_detects_added_removed_changed_nodes_and_edges() {
        let tmp = TempDir::new().unwrap();

        let keep = Node::new(NodeType::Function, "keep").with_file_path("a.rs");
        let removed = Node::new(NodeType::Function, "gone").with_file_path("a.rs");
        let keep_id = keep.id;
        let removed_id = removed.id;
        let base_edge = Edge::new(keep_id, removed_id, EdgeType::Calls);

        let base_path = write_snap(
            &tmp.path(),
            "base.bin",
            vec![keep.clone(), removed],
            vec![base_edge],
        );

        let changed = Node::new(NodeType::Function, "keep")
            .with_file_path("a.rs")
            .with_property("touch".into(), "1".into());
        let added = Node::new(NodeType::Function, "new_fn").with_file_path("b.rs");
        let changed_id = changed.id;
        let added_id = added.id;
        let head_edge = Edge::new(changed_id, added_id, EdgeType::Calls);

        let head_path = write_snap(
            &tmp.path(),
            "head.bin",
            vec![changed, added],
            vec![head_edge],
        );

        let base = SnapshotNodeStore::open(&base_path).unwrap();
        let head = SnapshotNodeStore::open(&head_path).unwrap();
        let mut sink = VecDiffSink::default();
        let stats = diff_snapshots(&base, &head, &mut sink).unwrap();

        assert_eq!(stats.nodes_added, 1);
        assert_eq!(stats.nodes_removed, 1);
        assert_eq!(stats.nodes_changed, 1);
        assert_eq!(stats.edges_added, 1);
        assert_eq!(stats.edges_removed, 1);

        assert!(sink.nodes.iter().any(|e| e.kind == NodeDeltaKind::Added));
        assert!(sink.nodes.iter().any(|e| e.kind == NodeDeltaKind::Removed));
        assert!(sink.nodes.iter().any(|e| e.kind == NodeDeltaKind::Changed));
        assert!(sink.edges.iter().any(|e| e.kind == EdgeDeltaKind::Added));
        assert!(sink.edges.iter().any(|e| e.kind == EdgeDeltaKind::Removed));
    }

    #[test]
    fn integration_two_fixture_snapshots() {
        let tmp = TempDir::new().unwrap();
        let n1 = Node::new(NodeType::Function, "main").with_file_path("main.rs");
        let n2 = Node::new(NodeType::Function, "helper").with_file_path("lib.rs");
        let id1 = n1.id;
        let id2 = n2.id;
        let base_path = write_snap(
            &tmp.path(),
            "base.bin",
            vec![n1, n2],
            vec![Edge::new(id1, id2, EdgeType::Calls)],
        );

        let n1b = Node::new(NodeType::Function, "main").with_file_path("main.rs");
        let n2b = Node::new(NodeType::Function, "helper").with_file_path("lib.rs");
        let n1b_id = n1b.id;
        let n2b_id = n2b.id;
        let head_path = write_snap(
            &tmp.path(),
            "head.bin",
            vec![n1b, n2b],
            vec![Edge::new(n1b_id, n2b_id, EdgeType::Calls)],
        );

        let pair = SnapshotPair::open(&base_path, &head_path).unwrap();
        let mut sink = VecDiffSink::default();
        let stats = diff_snapshots(&pair.base, &pair.head, &mut sink).unwrap();
        assert_eq!(stats.nodes_added, 0);
        assert_eq!(stats.nodes_removed, 0);
        assert_eq!(stats.edges_added, 0);
        assert_eq!(stats.edges_removed, 0);
    }
}
