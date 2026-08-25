//! Streaming log-structured compaction of a columnar snapshot with a delta segment.
//!
//! Pass 1 filters base nodes by invalidated file paths and appends delta nodes while
//! updating name/type indexes. Pass 2 streams base edges without building a full
//! topology `Vec`, keeping only endpoints present in the alive set.
//! Output is written to a temp path then atomically renamed.

use crate::columnar_snapshot::{ColumnarGraphMmap, EdgeRow, StringPool, write_columnar_assembled};
use crate::csr::{edge_type_from_u8, edge_type_to_u8};
use crate::normalize_path_str;
use crate::schema::{Edge, Node};
use crate::snapshot::MmappedGraphSnapshot;
use memmap2::Mmap;
use rgctl_error::{Error, Result};
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Extracted changes to merge into a base columnar snapshot.
#[derive(Debug, Default)]
pub struct DeltaSegment {
    /// Repo-relative (or normalized) file paths whose nodes must be dropped from the base.
    pub invalidated_files: HashSet<String>,
    /// Freshly extracted nodes from added/changed files.
    pub new_nodes: Vec<Node>,
    /// Freshly extracted edges from the delta extract (and optional relation rebuild).
    pub new_edges: Vec<Edge>,
}

impl DeltaSegment {
    /// Create an empty delta.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a path as invalidated (normalized separators).
    pub fn invalidate_file(&mut self, path: impl AsRef<str>) {
        self.invalidated_files
            .insert(normalize_path_str(path.as_ref()));
    }
}

/// Compacts a base [`ColumnarGraphMmap`] with a [`DeltaSegment`] into a new snapshot file.
pub struct GraphCompactor<'a> {
    base: &'a ColumnarGraphMmap,
    delta: DeltaSegment,
}

/// Statistics from a compaction run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactStats {
    /// Nodes kept from the base snapshot.
    pub nodes_kept: usize,
    /// Nodes dropped due to invalidated files.
    pub nodes_dropped: usize,
    /// Nodes appended from the delta.
    pub nodes_from_delta: usize,
    /// Edges kept from the base snapshot.
    pub edges_kept: usize,
    /// Edges dropped (endpoint not alive).
    pub edges_dropped: usize,
    /// Edges appended from the delta.
    pub edges_from_delta: usize,
    /// Content digest of the written snapshot.
    pub content_digest: String,
}

enum CompactNodeSource {
    Base(usize),
    Delta(usize),
}

struct CompactNodeEntry {
    id: Uuid,
    source: CompactNodeSource,
}

impl<'a> GraphCompactor<'a> {
    /// Create a compactor over a live mmap and owned delta.
    pub fn new(base: &'a ColumnarGraphMmap, delta: DeltaSegment) -> Self {
        Self { base, delta }
    }

    /// Compact to `output_path` via a sibling `.tmp` file and atomic rename.
    ///
    /// Base nodes copy extension bytes from mmap without bincode re-encode; delta nodes
    /// are encoded normally. Scratch dir is unused (kept for API stability).
    pub fn compact_to_path(self, output_path: &Path, _scratch_dir: &Path) -> Result<CompactStats> {
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut stats = CompactStats::default();
        let mut entries = Vec::new();

        for idx in 0..self.base.node_count() {
            if self
                .base
                .node_invalidated_at(idx, &self.delta.invalidated_files)?
            {
                stats.nodes_dropped += 1;
                continue;
            }
            let row_id = self.base.node_id_at(idx)?;
            entries.push(CompactNodeEntry {
                id: row_id,
                source: CompactNodeSource::Base(idx),
            });
            stats.nodes_kept += 1;
        }

        for (i, node) in self.delta.new_nodes.iter().enumerate() {
            entries.push(CompactNodeEntry {
                id: node.id,
                source: CompactNodeSource::Delta(i),
            });
            stats.nodes_from_delta += 1;
        }
        entries.sort_by_key(|entry| entry.id);

        let alive: HashSet<Uuid> = entries.iter().map(|entry| entry.id).collect();

        let mut hasher = blake3::Hasher::new();
        let mut strings = StringPool::new();
        let mut node_rows = Vec::with_capacity(entries.len());
        let mut extensions_blob = Vec::new();
        let mut name_index = std::collections::HashMap::new();
        let mut type_index = std::collections::HashMap::new();

        for entry in &entries {
            match entry.source {
                CompactNodeSource::Base(idx) => {
                    self.base.append_node_for_build(
                        idx,
                        &mut hasher,
                        &mut strings,
                        &mut extensions_blob,
                        &mut name_index,
                        &mut type_index,
                        &mut node_rows,
                    )?;
                }
                CompactNodeSource::Delta(i) => {
                    let node = &self.delta.new_nodes[i];
                    crate::columnar_snapshot::append_node_columnar(
                        node,
                        &mut hasher,
                        &mut strings,
                        &mut extensions_blob,
                        &mut name_index,
                        &mut type_index,
                        &mut node_rows,
                    )?;
                }
            }
        }

        let mut edge_meta: Vec<(Uuid, Uuid, u8)> = Vec::new();
        self.base.for_each_edge(|from, to, edge_type| {
            if alive.contains(&from) && alive.contains(&to) {
                edge_meta.push((from, to, edge_type_to_u8(edge_type)));
                stats.edges_kept += 1;
            } else {
                stats.edges_dropped += 1;
            }
            Ok(())
        })?;

        for edge in &self.delta.new_edges {
            if alive.contains(&edge.from) && alive.contains(&edge.to) {
                edge_meta.push((edge.from, edge.to, edge_type_to_u8(edge.edge_type)));
                stats.edges_from_delta += 1;
            }
        }

        edge_meta.sort_by(|a, b| a.cmp(b));

        let mut edge_rows = Vec::with_capacity(edge_meta.len());
        for (from, to, edge_type) in edge_meta {
            let canonical = Edge::new(from, to, edge_type_from_u8(edge_type)?);
            hasher.update(&crate::columnar_snapshot::edge_digest_bytes(&canonical)?);
            edge_rows.push(EdgeRow {
                from: *from.as_bytes(),
                to: *to.as_bytes(),
                edge_type,
                _pad: [0; 7],
            });
        }

        let content_digest = hasher.finalize().to_hex().to_string();
        stats.content_digest = content_digest.clone();

        let tmp_path = output_path.with_extension("bin.tmp");
        write_columnar_assembled(
            &tmp_path,
            &node_rows,
            &edge_rows,
            &strings,
            &extensions_blob,
            &name_index,
            &type_index,
            &content_digest,
        )?;

        if output_path.exists() {
            std::fs::remove_file(output_path)?;
        }
        std::fs::rename(&tmp_path, output_path)?;

        Ok(stats)
    }
}

/// Open a columnar snapshot from `path`, compact with `delta`, write atomically.
pub fn compact_snapshot_file(
    base_path: &Path,
    delta: DeltaSegment,
    output_path: &Path,
    scratch_dir: &Path,
) -> Result<CompactStats> {
    let file = File::open(base_path)?;
    // SAFETY: snapshot file is read-only for the duration of compaction; mapping covers
    // the on-disk columnar bytes only.
    let mmap = Arc::new(unsafe { Mmap::map(&file)? });
    let base = ColumnarGraphMmap::open(mmap)?;
    GraphCompactor::new(&base, delta).compact_to_path(output_path, scratch_dir)
}

/// Compact the default repo snapshot in place (via temp + rename).
pub fn compact_repo_snapshot(repo_root: &Path, delta: DeltaSegment) -> Result<CompactStats> {
    let snapshot_path = MmappedGraphSnapshot::default_path(repo_root);
    if !snapshot_path.exists() {
        return Err(Error::NotFound(format!(
            "snapshot not found at {}",
            snapshot_path.display()
        )));
    }
    let scratch = repo_root.join(".rgctl").join("compact-scratch");
    compact_snapshot_file(&snapshot_path, delta, &snapshot_path, &scratch)
}

#[cfg(test)]
use crate::schema::NodeType;

#[cfg(test)]
fn node_matches_invalidated(node: &Node, invalidated: &HashSet<String>) -> bool {
    let path = match (&node.file_path, node.node_type == NodeType::File) {
        (Some(fp), _) => fp.as_ref(),
        (None, true) => node.name.as_str(),
        _ => return false,
    };
    node_matches_invalidated_path(Some(path), node.name.as_str(), node.node_type, invalidated)
}

#[cfg(test)]
fn node_matches_invalidated_path(
    file_path: Option<&str>,
    name: &str,
    node_type: NodeType,
    invalidated: &HashSet<String>,
) -> bool {
    let path = match (file_path, node_type == NodeType::File) {
        (Some(fp), _) => fp,
        (None, true) => name,
        _ => return false,
    };

    let norm = normalize_path_str(path);
    if invalidated.contains(&norm) {
        return true;
    }

    norm.rsplit_once('/')
        .is_some_and(|(_, basename)| invalidated.contains(basename))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{EdgeType, NodeType};
    use crate::write_columnar_from_nodes_edges;
    use tempfile::TempDir;

    #[test]
    fn compact_drops_invalidated_file_and_appends_delta() {
        let keep = Node::new(NodeType::Function, "keep").with_file_path("a.rs");
        let drop_n = Node::new(NodeType::Function, "drop_me").with_file_path("b.rs");
        let keep_id = keep.id;
        let drop_id = drop_n.id;
        let e_keep = Edge::new(keep_id, keep_id, EdgeType::Calls);
        let e_drop = Edge::new(keep_id, drop_id, EdgeType::Calls);

        let tmp = TempDir::new().unwrap();
        let base_path = tmp.path().join("base.bin");
        write_columnar_from_nodes_edges(vec![keep, drop_n], vec![e_keep, e_drop], &base_path)
            .unwrap();

        let replacement = Node::new(NodeType::Function, "fresh").with_file_path("b.rs");
        let fresh_id = replacement.id;
        let mut delta = DeltaSegment::new();
        delta.invalidate_file("b.rs");
        delta.new_nodes.push(replacement);
        delta
            .new_edges
            .push(Edge::new(keep_id, fresh_id, EdgeType::Calls));

        let out = tmp.path().join("out.bin");
        let scratch = tmp.path().join("scratch");
        let stats = compact_snapshot_file(&base_path, delta, &out, &scratch).unwrap();

        assert_eq!(stats.nodes_kept, 1);
        assert_eq!(stats.nodes_dropped, 1);
        assert_eq!(stats.nodes_from_delta, 1);
        assert_eq!(stats.edges_kept, 1); // self-call on keep
        assert!(stats.edges_dropped >= 1);
        assert_eq!(stats.edges_from_delta, 1);

        let file = File::open(&out).unwrap();
        let mmap = Arc::new(unsafe { Mmap::map(&file).unwrap() });
        let col = ColumnarGraphMmap::open(mmap).unwrap();
        assert_eq!(col.node_count(), 2);
        let names: Vec<_> = (0..col.node_count())
            .map(|i| col.materialize_node_at(i).unwrap().name.to_string())
            .collect();
        assert!(names.iter().any(|n| n == "keep"));
        assert!(names.iter().any(|n| n == "fresh"));
        assert!(!names.iter().any(|n| n == "drop_me"));

        assert_eq!(col.find_nodes_by_name("fresh").unwrap().len(), 1);
        assert!(col.find_nodes_by_name("drop_me").unwrap().is_empty());
    }

    #[test]
    fn node_matches_invalidated_basename() {
        let node = Node::new(NodeType::Function, "foo").with_file_path("src/deep/b.rs");
        let mut invalidated = HashSet::new();
        invalidated.insert("b.rs".to_string());
        assert!(node_matches_invalidated(&node, &invalidated));
    }
}
