//! Cold node-payload access for discover after topology is extracted.
//!
//! After early columnar mmap write + CSR build, algorithms should prefer
//! [`StructuralTopology`] for edges and this store for sparse `get_node` lookups
//! so the fat [`MemoryBackend`] can be dropped (or its edge storage released).

use rgctl_error::Result;
use rgctl_graph::schema::{Node, NodeType};
use rgctl_graph::snapshot::SnapshotNodeStore;
use std::path::Path;
use uuid::Uuid;

/// Read-only cold metadata backed by the on-disk columnar snapshot.
pub struct ColdMetadataDb {
    store: SnapshotNodeStore,
}

impl ColdMetadataDb {
    /// Open the graph snapshot already written under `path` (usually `.rgctl/graph.snapshot.bin`).
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            store: SnapshotNodeStore::open(path)?,
        })
    }

    /// Open the default snapshot path for a repository root.
    pub fn open_from_repo(repo_root: &Path) -> Result<Self> {
        Ok(Self {
            store: SnapshotNodeStore::open_from_repo(repo_root)?,
        })
    }

    /// Underlying snapshot store.
    pub fn store(&self) -> &SnapshotNodeStore {
        &self.store
    }

    /// Lookup a node by UUID (may deserialize extension blob from mmap).
    pub fn get_node(&self, id: Uuid) -> Result<Option<Node>> {
        self.store.get_node(id)
    }

    /// Node count from snapshot header.
    pub fn node_count(&self) -> usize {
        self.store.node_count()
    }

    /// Edge count from snapshot header.
    pub fn edge_count(&self) -> usize {
        self.store.edge_count()
    }

    /// Collect nodes of a given type (linear scan — prefer for small result sets).
    pub fn collect_nodes_by_type(&self, node_type: NodeType) -> Result<Vec<Node>> {
        let mut out = Vec::new();
        for id in self.store.all_node_ids() {
            if let Some(node) = self.store.get_node(id)? {
                if node.node_type == node_type {
                    out.push(node);
                }
            }
        }
        Ok(out)
    }
}
