//! Deduplicated source files for dashboard slice/dataflow views.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

pub const SOURCE_INDEX_FILE: &str = "sources_index.json";
pub const SOURCE_FILES_DIR: &str = "sources";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceIndexPayload {
    pub schema_version: u32,
    pub files: Vec<SourceFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFileEntry {
    pub source_id: String,
    pub file_path: String,
    pub total_lines: usize,
}

/// Stable short id for a repository source path.
pub fn source_id_for_path(file_path: &str) -> String {
    blake3::hash(file_path.as_bytes())
        .to_hex()
        .chars()
        .take(16)
        .collect()
}

/// Write deduplicated source text files and return id + line count for `file_path`.
///
/// File IO happens outside the cache lock so Rayon workers are not serialized on reads.
pub fn ensure_source_file(
    out_dir: &Path,
    file_path: &str,
    cache: &Mutex<HashMap<String, SourceFileEntry>>,
) -> Option<SourceFileEntry> {
    if let Ok(guard) = cache.lock()
        && let Some(entry) = guard.get(file_path)
    {
        return Some(entry.clone());
    }
    let text = fs::read_to_string(file_path).ok()?;
    let total_lines = text.lines().count().max(1);
    let source_id = source_id_for_path(file_path);
    let sources_dir = out_dir.join(SOURCE_FILES_DIR);
    fs::create_dir_all(&sources_dir).ok()?;
    let dest = sources_dir.join(format!("{source_id}.txt"));
    if !dest.is_file() {
        fs::write(&dest, &text).ok()?;
    }
    let entry = SourceFileEntry {
        source_id,
        file_path: file_path.to_string(),
        total_lines,
    };
    if let Ok(mut guard) = cache.lock() {
        let stored = guard
            .entry(file_path.to_string())
            .or_insert_with(|| entry.clone());
        return Some(stored.clone());
    }
    Some(entry)
}

pub fn write_source_index(out_dir: &Path, entries: &[SourceFileEntry]) -> Result<(), String> {
    let mut files = entries.to_vec();
    files.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    let payload = SourceIndexPayload {
        schema_version: 1,
        files,
    };
    super::export_util::write_json_compact(&out_dir.join(SOURCE_INDEX_FILE), &payload)
}
