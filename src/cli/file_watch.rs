//! Debounced filesystem watcher that triggers incremental graph updates.

use crate::languages::registry::LanguageRegistry;
use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use rgctl_project_config::RgctlConfig;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

const DEFAULT_CASCADE_DEPTH: usize = 1;

/// Directory name segments that must never trigger an update.
const SKIP_DIR_NAMES: &[&str] = &[".git", "target", "node_modules", ".rgctl"];

/// Start a background watcher thread. Returns a join handle (detached by caller).
pub fn spawn_repo_watcher(repo: PathBuf) -> Result<thread::JoinHandle<()>> {
    let debounce_ms = RgctlConfig::load(&repo)
        .unwrap_or_default()
        .watch
        .debounce_ms;
    let registry = LanguageRegistry::new();
    let extensions: HashSet<String> = registry
        .supported_extensions()
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .collect();

    let (tx, rx) = mpsc::channel::<PathBuf>();
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        match res {
            Ok(event) => {
                if !is_interesting_event(&event.kind) {
                    return;
                }
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
            Err(err) => warn!("rgctl watch: notify error: {err}"),
        }
    })
    .context("create filesystem watcher")?;

    watcher
        .watch(&repo, RecursiveMode::Recursive)
        .with_context(|| format!("watch {}", repo.display()))?;

    // Keep watcher alive for the thread lifetime.
    let watcher = Arc::new(Mutex::new(watcher));
    let watcher_keep = Arc::clone(&watcher);

    let handle = thread::Builder::new()
        .name("rgctl-file-watch".into())
        .spawn(move || {
            let _keeper = watcher_keep;
            watch_loop(repo, rx, extensions, Duration::from_millis(debounce_ms));
        })
        .context("spawn watch thread")?;

    Ok(handle)
}

fn is_interesting_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
    )
}

fn watch_loop(
    repo: PathBuf,
    rx: mpsc::Receiver<PathBuf>,
    extensions: HashSet<String>,
    debounce: Duration,
) {
    let mut pending: HashSet<String> = HashSet::new();
    let mut deadline: Option<Instant> = None;

    loop {
        let timeout = deadline.map(|d| d.saturating_duration_since(Instant::now()));
        let recv = match timeout {
            Some(t) if !t.is_zero() => rx.recv_timeout(t),
            Some(_) => Err(RecvTimeoutError::Timeout),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        match recv {
            Ok(path) => {
                if let Some(rel) = filter_watch_path(&repo, &path, &extensions) {
                    pending.insert(rel);
                    deadline = Some(Instant::now() + debounce);
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                if pending.is_empty() {
                    deadline = None;
                    continue;
                }
                let batch: Vec<String> = pending.drain().collect();
                deadline = None;
                info!("watch: updating {} file(s)", batch.len());
                // Extract/compact on this dedicated watch thread (not the tokio serve runtime).
                match super::update::update_paths(&repo, &batch, DEFAULT_CASCADE_DEPTH) {
                    Ok(result) => {
                        info!(
                            "watch: incremental update done (affected={}, +{}/-{} nodes)",
                            result.files_affected(),
                            result.nodes_added,
                            result.nodes_removed
                        );
                    }
                    Err(err) => warn!("watch: incremental update failed: {err:#}"),
                }
                // Drain any events buffered during the update into the next debounce window.
                while let Ok(path) = rx.try_recv() {
                    if let Some(rel) = filter_watch_path(&repo, &path, &extensions) {
                        pending.insert(rel);
                    }
                }
                if !pending.is_empty() {
                    deadline = Some(Instant::now() + debounce);
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                debug!("watch: channel closed");
                break;
            }
        }
    }
}

fn filter_watch_path(repo: &Path, path: &Path, extensions: &HashSet<String>) -> Option<String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    let rel = abs.strip_prefix(repo).ok()?;
    if rel.components().any(|c| {
        let s = c.as_os_str().to_string_lossy();
        SKIP_DIR_NAMES.iter().any(|skip| *skip == s.as_ref())
    }) {
        return None;
    }
    // Skip editor swap / temp noise
    if let Some(name) = rel.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') || name.ends_with('~') || name.ends_with(".swp") {
            return None;
        }
    }
    let ext = rel
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())?;
    if !extensions.contains(&ext) {
        return None;
    }
    Some(rel.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn filter_skips_target_and_non_source() {
        let repo = PathBuf::from("/tmp/repo");
        let mut exts = HashSet::new();
        exts.insert("rs".into());
        assert!(
            filter_watch_path(&repo, Path::new("/tmp/repo/target/foo.rs"), &exts).is_none()
        );
        assert!(filter_watch_path(&repo, Path::new("/tmp/repo/README.md"), &exts).is_none());
        assert_eq!(
            filter_watch_path(&repo, Path::new("/tmp/repo/src/lib.rs"), &exts).as_deref(),
            Some("src/lib.rs")
        );
    }
}
