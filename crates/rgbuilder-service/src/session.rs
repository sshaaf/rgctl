//! Warm graph session for command execution.

use crate::error::{Result, ServiceError};
use rgbuilder_graph::CodeGraph;
use rgbuilder_graph::snapshot::SNAPSHOT_FILE;
use std::path::{Path, PathBuf};

/// Repository session: lazy graph load and digest reload.
pub struct Session {
    repo: PathBuf,
    graph: Option<CodeGraph>,
    digest: Option<String>,
}

impl Session {
    /// Bind to a repository root (where `.rgbuilder/` lives).
    #[must_use]
    pub fn new(repo: impl Into<PathBuf>) -> Self {
        let repo = repo.into();
        let _ = rgbuilder_graph::paths::ensure_artifact_dir_migrated(&repo);
        Self {
            repo,
            graph: None,
            digest: None,
        }
    }

    /// Session root.
    #[must_use]
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// Snapshot path under the artifact dir.
    #[must_use]
    pub fn snapshot_path(&self) -> PathBuf {
        rgbuilder_graph::paths::artifact_path(&self.repo, SNAPSHOT_FILE)
    }

    /// Whether a graph snapshot (or legacy db) exists on disk.
    #[must_use]
    pub fn graph_ready(&self) -> bool {
        self.snapshot_path().exists()
            || rgbuilder_graph::paths::artifact_path(&self.repo, "graph.db").exists()
            || rgbuilder_graph::paths::artifact_path(&self.repo, "graph.json").exists()
    }

    /// Load or return the cached graph. Errors if no snapshot exists.
    pub fn load_graph(&mut self) -> Result<&CodeGraph> {
        self.reload_if_stale()?;
        if self.graph.is_none() {
            let graph = self.open_graph()?;
            self.digest = graph_digest_for(&self.repo);
            self.graph = Some(graph);
        }
        self.graph
            .as_ref()
            .ok_or_else(|| ServiceError::Failed("graph not loaded".into()))
    }

    fn reload_if_stale(&mut self) -> Result<()> {
        let current = graph_digest_for(&self.repo);
        if self.graph.is_some() && current != self.digest {
            self.graph = None;
            self.digest = current;
        }
        Ok(())
    }

    fn open_graph(&self) -> Result<CodeGraph> {
        let snapshot = self.snapshot_path();
        if snapshot.exists() {
            return CodeGraph::open_snapshot(&snapshot).map_err(ServiceError::from);
        }
        let db = rgbuilder_graph::paths::artifact_path(&self.repo, "graph.db");
        if db.exists() {
            let json = std::fs::read_to_string(&db).map_err(rgbuilder_error::Error::from)?;
            return CodeGraph::import_json(&json).map_err(ServiceError::from);
        }
        let legacy = rgbuilder_graph::paths::artifact_path(&self.repo, "graph.json");
        if legacy.exists() {
            let json = std::fs::read_to_string(&legacy).map_err(rgbuilder_error::Error::from)?;
            return CodeGraph::import_json(&json).map_err(ServiceError::from);
        }
        Err(ServiceError::Failed(format!(
            "Graph not found at {} (run `rgctl discover` first)",
            db.display()
        )))
    }
}

pub(crate) fn graph_digest_for(repo: &Path) -> Option<String> {
    let path = rgbuilder_graph::paths::artifact_path(repo, SNAPSHOT_FILE);
    let store = rgbuilder_graph::SnapshotNodeStore::open(&path).ok()?;
    store.content_digest().ok().map(str::to_string)
}
