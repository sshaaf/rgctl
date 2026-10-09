//! Session graph status: cheap freshness / presence check (no rediscover).

use super::context::CliContext;
use super::OutputFormat;
use crate::discovery::{DiscoveryConfig, FileDiscoverer};
use crate::languages::registry::LanguageRegistry;
use anyhow::Result;
use rgctl_incremental::FileTracker;
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;

const STATUS_SCHEMA_VERSION: u32 = 2;
const DIRTY_SAMPLE_LIMIT: usize = 10;

#[derive(Debug, Serialize)]
struct SessionStatus {
    schema_version: u32,
    command: &'static str,
    /// `ok` | `missing`
    status: &'static str,
    repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nodes: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edges: Option<usize>,
    kantra_findings: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    kantra_findings_path: Option<String>,
    /// Whether discoverable sources match `file_hashes.json` (None when snapshot missing).
    #[serde(skip_serializing_if = "Option::is_none")]
    index_current: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty_files: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_added: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_changed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files_deleted: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty_sample: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Debug, Default)]
struct Staleness {
    index_current: bool,
    dirty_files: usize,
    files_added: usize,
    files_changed: usize,
    files_deleted: usize,
    dirty_sample: Vec<String>,
}

fn kantra_findings_path(repo: &Path) -> std::path::PathBuf {
    rgctl_graph::paths::artifact_path(repo, "kantra_findings.json")
}

/// Compute working-tree vs tracker freshness without extract/compact.
fn compute_staleness(repo: &Path) -> Staleness {
    let registry: Arc<rgctl_registry::LanguageRegistry> = LanguageRegistry::new().into();
    let discoverer = FileDiscoverer::with_config(registry, DiscoveryConfig::default());
    let files = match discoverer.discover(repo) {
        Ok(f) => f,
        Err(_) => {
            return Staleness {
                index_current: true,
                ..Default::default()
            };
        }
    };
    let tracker = FileTracker::load(repo).unwrap_or_else(|_| FileTracker::new(repo));
    let changes = match tracker.detect_changes(&files) {
        Ok(c) => c,
        Err(_) => {
            return Staleness {
                index_current: true,
                ..Default::default()
            };
        }
    };
    let files_added = changes.added.len();
    let files_changed = changes.changed.len();
    let files_deleted = changes.deleted.len();
    let dirty_files = files_added + files_changed + files_deleted;
    let mut dirty_sample: Vec<String> = changes
        .added
        .iter()
        .chain(changes.changed.iter())
        .chain(changes.deleted.iter())
        .take(DIRTY_SAMPLE_LIMIT)
        .cloned()
        .collect();
    dirty_sample.sort();
    Staleness {
        index_current: dirty_files == 0,
        dirty_files,
        files_added,
        files_changed,
        files_deleted,
        dirty_sample,
    }
}

/// `rgctl status` — report whether `.rgctl/` has a usable snapshot.
pub fn run_status(ctx: &CliContext) -> Result<()> {
    let findings = kantra_findings_path(&ctx.repo);
    let kantra_present = findings.is_file();
    let kantra_path = kantra_present.then(|| findings.display().to_string());

    let session = ctx.snapshot_session()?;
    let payload = match session {
        Some(s) => {
            let stale = compute_staleness(&ctx.repo);
            let message = if stale.index_current {
                None
            } else {
                Some(format!(
                    "Index not current ({} dirty source file(s)); run `rgctl update` or use `rgctl serve --watch`",
                    stale.dirty_files
                ))
            };
            SessionStatus {
                schema_version: STATUS_SCHEMA_VERSION,
                command: "status",
                status: "ok",
                repo: ctx.repo.display().to_string(),
                snapshot: Some(
                    rgctl_graph::paths::artifact_path(
                        &ctx.repo,
                        rgctl_graph::snapshot::SNAPSHOT_FILE,
                    )
                    .display()
                    .to_string(),
                ),
                digest: Some(s.digest.to_string()),
                nodes: Some(s.store.node_count()),
                edges: Some(s.store.edge_count()),
                kantra_findings: kantra_present,
                kantra_findings_path: kantra_path,
                index_current: Some(stale.index_current),
                dirty_files: Some(stale.dirty_files),
                files_added: Some(stale.files_added),
                files_changed: Some(stale.files_changed),
                files_deleted: Some(stale.files_deleted),
                dirty_sample: if stale.dirty_sample.is_empty() {
                    None
                } else {
                    Some(stale.dirty_sample)
                },
                message,
            }
        }
        None => SessionStatus {
            schema_version: STATUS_SCHEMA_VERSION,
            command: "status",
            status: "missing",
            repo: ctx.repo.display().to_string(),
            snapshot: None,
            digest: None,
            nodes: None,
            edges: None,
            kantra_findings: kantra_present,
            kantra_findings_path: kantra_path,
            index_current: None,
            dirty_files: None,
            files_added: None,
            files_changed: None,
            files_deleted: None,
            dirty_sample: None,
            message: Some("Graph snapshot not found; run `rgctl discover` first".into()),
        },
    };

    if ctx.format == OutputFormat::Json {
        let v = serde_json::to_value(&payload)?;
        ctx.emit_json_value(&v)?;
    } else if payload.status == "ok" {
        let current = payload
            .index_current
            .map(|c| if c { "yes" } else { "no" })
            .unwrap_or("?");
        ctx.stdout_line(&format!(
            "status=ok nodes={} edges={} digest={} index_current={}{}{}",
            payload.nodes.unwrap_or(0),
            payload.edges.unwrap_or(0),
            payload.digest.as_deref().unwrap_or("?"),
            current,
            if payload.kantra_findings {
                " kantra_findings=yes"
            } else {
                ""
            },
            payload
                .message
                .as_ref()
                .map(|m| format!(" — {m}"))
                .unwrap_or_default()
        ))?;
    } else {
        ctx.stdout_line(
            payload
                .message
                .as_deref()
                .unwrap_or("Graph snapshot missing"),
        )?;
    }

    if payload.status == "missing" {
        anyhow::bail!("graph snapshot missing");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{PipelineConfig, ProcessingPipeline};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn staleness_detects_dirty_source() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        let pipeline = ProcessingPipeline::with_config(
            LanguageRegistry::new().into(),
            PipelineConfig {
                show_progress: false,
                ..PipelineConfig::default()
            },
        );
        let (graph, _) = pipeline.process_repository(root).unwrap();
        graph.save_to_repo(root).unwrap();
        let mut tracker = FileTracker::new(root);
        tracker
            .index_files(&[root.join("src/lib.rs")], &graph)
            .unwrap();
        tracker.save().unwrap();

        let clean = compute_staleness(root);
        assert!(clean.index_current, "expected clean index");

        fs::write(root.join("src/lib.rs"), "pub fn a() {}\npub fn b() {}\n").unwrap();
        let dirty = compute_staleness(root);
        assert!(!dirty.index_current);
        assert!(dirty.dirty_files >= 1);
    }
}
