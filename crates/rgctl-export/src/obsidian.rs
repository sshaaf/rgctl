//! Obsidian vault export from markdown context graph nodes.

use rgctl_error::Result;
use rgctl_graph::backend::MemoryBackend;
use rgctl_graph::content_store::ContentStore;
use rgctl_graph::schema::{EdgeType, Node, NodeType};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

/// Max length for a directory path segment (macOS filename limit is 255 bytes).
const MAX_PATH_SEGMENT_LEN: usize = 200;
/// Max stem length before `.md` on note files.
const MAX_NOTE_STEM_LEN: usize = 252;
/// Max buffered note jobs between producer and writer workers.
const NOTE_JOB_CHANNEL_CAPACITY: usize = 256;
/// Cap parallel filesystem writers to avoid over-saturating IO.
const MAX_WRITER_THREADS: usize = 8;

/// Stats from an Obsidian vault export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ObsidianExportStats {
    /// Notes written.
    pub notes_written: usize,
    /// Wikilinks emitted from `REFERENCES` edges.
    pub links_written: usize,
}

/// Export heading sections as Obsidian-compatible markdown notes.
pub fn export_obsidian_vault(
    backend: &MemoryBackend,
    content_store: &ContentStore,
    output_dir: &Path,
    repo_root: &Path,
) -> Result<ObsidianExportStats> {
    fs::create_dir_all(output_dir)?;

    let repo_prefix = repo_root
        .canonicalize()
        .unwrap_or_else(|_| repo_root.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");

    let mut headings: Vec<(uuid::Uuid, String)> = Vec::new();
    backend.for_each_node(|node| {
        if node.node_type == NodeType::Module && node.get_property("kind") == Some("heading") {
            if let Some(qn) = node.qualified_name.as_ref() {
                headings.push((node.id, qn.to_string()));
            }
        }
    })?;

    let references_by_heading = build_reference_index(backend)?;
    let notes_written = Arc::new(AtomicUsize::new(0));
    let links_written = Arc::new(AtomicUsize::new(0));
    let first_error: Arc<Mutex<Option<rgctl_error::Error>>> = Arc::new(Mutex::new(None));
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(MAX_WRITER_THREADS)
        .max(1);
    let (job_tx, job_rx) = crossbeam_channel::bounded::<NoteJob>(NOTE_JOB_CHANNEL_CAPACITY);
    let mut workers = Vec::with_capacity(worker_count);
    for _ in 0..worker_count {
        let rx = job_rx.clone();
        let notes = Arc::clone(&notes_written);
        let links = Arc::clone(&links_written);
        let err = Arc::clone(&first_error);
        workers.push(thread::spawn(move || {
            for job in rx.iter() {
                if let Err(e) = write_note_job(job, &notes, &links) {
                    let mut guard = err.lock().expect("lock error slot");
                    if guard.is_none() {
                        *guard = Some(e);
                    }
                    break;
                }
            }
        }));
    }
    drop(job_rx);

    for (id, qn) in headings {
        if first_error.lock().expect("lock error slot").is_some() {
            break;
        }
        let body = backend
            .with_node(id, |node| resolve_body(node, content_store))?
            .flatten()
            .unwrap_or_default();

        let note_rel = backend
            .with_node(id, |node| note_relpath_for_heading(node, &repo_prefix))?
            .flatten()
            .unwrap_or_else(|| note_relpath(&qn, &repo_prefix));

        let mut wikilinks: Vec<String> = Vec::new();
        if let Some(targets) = references_by_heading.get(&id) {
            for &target in targets {
                if let Ok(label) = backend
                    .with_node(target, |n| obsidian_wikilink_for_node(n, &repo_prefix))
                    .map(|inner| inner.flatten())
                    && let Some(label) = label
                {
                    wikilinks.push(format!("[[{label}]]"));
                }
            }
        }
        let note_path = output_dir.join(&note_rel);
        let level = backend
            .with_node(id, |n| n.get_property("level").map(|s| s.to_string()))?
            .flatten()
            .unwrap_or_default();

        let mut out = String::new();
        out.push_str("---\n");
        out.push_str(&format!("qualified_name: \"{qn}\"\n"));
        if !level.is_empty() {
            out.push_str(&format!("level: \"{level}\"\n"));
        }
        out.push_str("---\n\n");
        if !body.is_empty() {
            out.push_str(&body);
            out.push('\n');
        }
        let link_count = wikilinks.len();
        for link in wikilinks {
            out.push_str(&link);
            out.push('\n');
        }
        job_tx
            .send(NoteJob {
                note_path,
                content: out,
                links_written: link_count,
            })
            .map_err(|e| rgctl_error::Error::GraphError(format!("queue note job: {e}")))?;
    }
    drop(job_tx);
    for worker in workers {
        let _ = worker.join();
    }
    if let Some(err) = first_error.lock().expect("lock error slot").take() {
        return Err(err);
    }

    Ok(ObsidianExportStats {
        notes_written: notes_written.load(Ordering::Relaxed),
        links_written: links_written.load(Ordering::Relaxed),
    })
}

fn build_reference_index(backend: &MemoryBackend) -> Result<HashMap<uuid::Uuid, Vec<uuid::Uuid>>> {
    let mut refs = HashMap::new();
    backend.for_each_edge(|edge| {
        if edge.edge_type == EdgeType::References {
            refs.entry(edge.from).or_insert_with(Vec::new).push(edge.to);
        }
    })?;
    Ok(refs)
}

#[derive(Debug)]
struct NoteJob {
    note_path: std::path::PathBuf,
    content: String,
    links_written: usize,
}

fn write_note_job(
    job: NoteJob,
    notes_written: &AtomicUsize,
    links_written: &AtomicUsize,
) -> Result<()> {
    if let Some(parent) = job.note_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            rgctl_error::Error::GraphError(format!("create_dir {}: {e}", parent.display()))
        })?;
    }
    fs::write(&job.note_path, job.content).map_err(|e| {
        rgctl_error::Error::GraphError(format!("write note {}: {e}", job.note_path.display()))
    })?;
    notes_written.fetch_add(1, Ordering::Relaxed);
    links_written.fetch_add(job.links_written, Ordering::Relaxed);
    Ok(())
}

fn obsidian_wikilink_for_node(node: &Node, repo_prefix: &str) -> Option<String> {
    if node.node_type == NodeType::Module && node.get_property("kind") == Some("heading") {
        return note_relpath_for_heading(node, repo_prefix)
            .as_deref()
            .map(obsidian_link_from_relpath);
    }
    if let Some(qn) = node.qualified_name.as_ref() {
        return Some(obsidian_link_from_relpath(&note_relpath(qn, repo_prefix)));
    }
    if node.node_type == NodeType::File {
        let rel = strip_repo_prefix(
            node.file_path
                .as_ref()
                .map(|s| s.as_ref())
                .unwrap_or(node.name.as_ref()),
            repo_prefix,
        );
        let base = rel
            .strip_suffix(".md")
            .or_else(|| rel.strip_suffix(".mdx"))
            .unwrap_or(&rel);
        return Some(base.to_string());
    }
    None
}

fn note_relpath_for_heading(node: &Node, repo_prefix: &str) -> Option<String> {
    let file = node
        .file_path
        .as_ref()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let rel_file = strip_repo_prefix(&file, repo_prefix);
    let qn = node.qualified_name.as_ref()?;
    let fragment = qn.split('#').nth(1).unwrap_or("");
    Some(note_relpath_from_parts(&rel_file, fragment, qn))
}

fn obsidian_link_from_relpath(note_rel: &str) -> String {
    note_rel.strip_suffix(".md").unwrap_or(note_rel).to_string()
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

fn strip_repo_prefix(path: &str, repo_prefix: &str) -> String {
    let normalized_path = normalize_path(path);
    let normalized_repo = normalize_path(repo_prefix);
    let rel_path = normalized_path
        .strip_prefix(&normalized_repo)
        .unwrap_or(&normalized_path);
    path_to_relative_string(rel_path)
}

fn note_relpath_from_parts(file_path: &str, fragment: &str, hash_seed: &str) -> String {
    let file = file_path.replace('\\', "/");
    let file_no_ext = file
        .strip_suffix(".md")
        .or_else(|| file.strip_suffix(".mdx"))
        .unwrap_or(&file);
    let base = sanitize_relpath(file_no_ext, hash_seed);
    if fragment.is_empty() {
        format!("{base}.md")
    } else {
        let frag = sanitize_path_component(fragment, hash_seed, MAX_NOTE_STEM_LEN);
        format!("{base}/{frag}.md")
    }
}

fn sanitize_relpath(path: &str, hash_seed: &str) -> String {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            sanitize_path_component(seg, &format!("{hash_seed}/{seg}"), MAX_PATH_SEGMENT_LEN)
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn note_relpath(qualified_name: &str, repo_prefix: &str) -> String {
    let normalized = qualified_name.replace('\\', "/");
    if let Some((file, frag)) = normalized.split_once('#') {
        let rel_file = strip_repo_prefix(file, repo_prefix);
        note_relpath_from_parts(&rel_file, frag, qualified_name)
    } else {
        format!(
            "{}.md",
            sanitize_path_component(&normalized, qualified_name, MAX_NOTE_STEM_LEN)
        )
    }
}

fn sanitize_path_component(raw: &str, hash_seed: &str, max_len: usize) -> String {
    let mut cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect();
    while cleaned.contains("--") {
        cleaned = cleaned.replace("--", "-");
    }
    cleaned = cleaned.trim_matches('-').to_string();
    if cleaned.is_empty() {
        cleaned = "section".to_string();
    }
    if cleaned.len() > max_len {
        let hash = short_hash(hash_seed);
        let keep = max_len.saturating_sub(hash.len() + 1);
        let truncated = truncate_utf8_bytes(&cleaned, keep);
        let truncated = truncated.trim_end_matches('-');
        format!("{truncated}-{hash}")
    } else {
        cleaned
    }
}

fn normalize_path(input: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for component in Path::new(input).components() {
        match component {
            Component::Prefix(prefix) => out = PathBuf::from(prefix.as_os_str()),
            Component::RootDir => out.push(std::path::MAIN_SEPARATOR_STR),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn path_to_relative_string(path: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().to_string()),
            Component::ParentDir => {
                let _ = parts.pop();
            }
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}

fn truncate_utf8_bytes(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = 0usize;
    for (idx, ch) in text.char_indices() {
        let next = idx + ch.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    &text[..end]
}

fn short_hash(seed: &str) -> String {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    format!("{:08x}", hasher.finish() & 0xffffffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_repo_prefix_removes_dot_slash_after_root() {
        let root = "/repo/k8s-website";
        let path = "/repo/k8s-website/./docs/guide.md";
        assert_eq!(strip_repo_prefix(path, root), "docs/guide.md");
    }

    #[test]
    fn sanitize_truncates_overlong_fragments_with_stable_hash() {
        let long = "a".repeat(443);
        let seed = "docs/post.md#full";
        let once = sanitize_path_component(&long, seed, MAX_NOTE_STEM_LEN);
        let again = sanitize_path_component(&long, seed, MAX_NOTE_STEM_LEN);
        assert_eq!(once, again);
        assert!(once.chars().count() <= MAX_NOTE_STEM_LEN);
        assert!(once.contains('-'));
    }

    #[test]
    fn note_relpath_from_parts_limits_component_length() {
        let frag = "one-of-the-advantages-that-kubernetes-provides-is-the-ability-to-manage-various-environments-easier-and-better-than-traditional-deployment-strategies-for-most-nontrivial-applications-you-have-test-data-staging-and-production-test-data-staging-and-production";
        let rel = note_relpath_from_parts(
            "blog/_posts/2015/using-kubernetes-namespaces-to-manage.md",
            frag,
            "blog/_posts/2015/using-kubernetes-namespaces-to-manage.md#full",
        );
        let max_comp = rel.split('/').map(|c| c.len()).max().unwrap_or(0);
        assert!(max_comp <= 255);
    }

    #[test]
    fn sanitize_truncates_multibyte_segments_by_byte_length() {
        let long = "界".repeat(200);
        let out = sanitize_path_component(&long, "docs/guide.md#section", 252);
        assert!(out.len() <= 252);
    }

    #[test]
    fn strip_repo_prefix_normalizes_parent_segments() {
        let root = "/repo/site";
        let path = "/repo/site/docs/../guide/./index.md";
        assert_eq!(strip_repo_prefix(path, root), "guide/index.md");
    }
}
