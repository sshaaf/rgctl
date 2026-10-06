//! Content blob vault for truncated markdown bodies and large file payloads.

use rgctl_error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Inline property cap aligned with markdown extraction (`INLINE_BODY_MAX_BYTES`).
pub const INLINE_BODY_MAX_BYTES: usize = 32_768;

/// Default filename under `.rgctl/`.
pub const CONTENT_STORE_FILE: &str = "content_store.bin";

/// BLAKE3 hex digest of UTF-8 text (same as markdown `body_hash`).
pub fn hash_text(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex().to_string()
}

/// BLAKE3 hex digest of raw bytes (file payloads).
pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Hash-keyed blob store for out-of-line document bodies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContentStore {
    blobs: HashMap<String, Vec<u8>>,
    #[serde(skip)]
    cache_file: Option<PathBuf>,
}

impl ContentStore {
    /// Empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store backed by a cache file path.
    pub fn with_cache_file(cache_file: PathBuf) -> Self {
        Self {
            blobs: HashMap::new(),
            cache_file: Some(cache_file),
        }
    }

    /// Insert UTF-8 text under `hash`.
    pub fn insert_str(&mut self, hash: &str, text: &str) {
        self.blobs
            .insert(hash.to_string(), text.as_bytes().to_vec());
    }

    /// Insert raw bytes under `hash`.
    pub fn insert_bytes(&mut self, hash: &str, bytes: Vec<u8>) {
        self.blobs.insert(hash.to_string(), bytes);
    }

    /// Merge hash→text blobs from extraction.
    pub fn merge_text_blobs(&mut self, blobs: &HashMap<String, String>) {
        for (hash, text) in blobs {
            self.insert_str(hash, text);
        }
    }

    /// Look up raw bytes.
    pub fn get(&self, hash: &str) -> Option<&[u8]> {
        self.blobs.get(hash).map(|v| v.as_slice())
    }

    /// Look up UTF-8 text.
    pub fn get_str(&self, hash: &str) -> Option<&str> {
        self.get(hash).and_then(|b| std::str::from_utf8(b).ok())
    }

    /// Number of stored blobs.
    pub fn len(&self) -> usize {
        self.blobs.len()
    }

    /// True when no blobs are stored.
    pub fn is_empty(&self) -> bool {
        self.blobs.is_empty()
    }

    /// Persist to the configured cache file (bincode).
    pub fn save(&self) -> Result<()> {
        let Some(path) = &self.cache_file else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = bincode::serialize(&self.blobs)
            .map_err(|e| Error::SerdeError(format!("content store encode: {e}")))?;
        let tmp = path.with_extension("bin.tmp");
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    /// Load from disk, or return empty when missing.
    pub fn load(cache_file: PathBuf) -> Result<Self> {
        if cache_file.exists() {
            let bytes = std::fs::read(&cache_file)?;
            match decode_blob_map(&bytes) {
                Ok(blobs) => Ok(Self {
                    blobs,
                    cache_file: Some(cache_file),
                }),
                Err(err) => {
                    tracing::warn!(
                        path = %cache_file.display(),
                        error = %err,
                        "content store unreadable — starting empty"
                    );
                    Ok(Self::with_cache_file(cache_file))
                }
            }
        } else {
            Ok(Self::with_cache_file(cache_file))
        }
    }

    /// Default path under a repository root.
    pub fn default_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".rgctl").join(CONTENT_STORE_FILE)
    }
}

fn decode_blob_map(bytes: &[u8]) -> Result<HashMap<String, Vec<u8>>> {
    if bytes.is_empty() {
        return Ok(HashMap::new());
    }
    let mut cur = 0usize;
    let count = read_bincode_u64(bytes, &mut cur)?;
    let max_entries = bytes.len() / 16;
    if count as usize > max_entries {
        return Err(Error::SerdeError(format!(
            "content store entry count {count} exceeds file size"
        )));
    }
    let mut blobs = HashMap::new();
    blobs.try_reserve(count as usize).map_err(|_| {
        Error::SerdeError(format!("content store entry count {count} too large"))
    })?;
    for _ in 0..count {
        let klen = read_bincode_u64(bytes, &mut cur)? as usize;
        let key_bytes = read_exact_slice(bytes, &mut cur, klen, "key")?;
        let key = std::str::from_utf8(key_bytes)
            .map_err(|e| Error::SerdeError(format!("content store key: {e}")))?
            .to_string();
        let vlen = read_bincode_u64(bytes, &mut cur)? as usize;
        let val = read_exact_slice(bytes, &mut cur, vlen, "blob")?.to_vec();
        blobs.insert(key, val);
    }
    Ok(blobs)
}

fn read_bincode_u64(bytes: &[u8], cur: &mut usize) -> Result<u64> {
    if *cur + 8 > bytes.len() {
        return Err(Error::SerdeError("content store truncated".into()));
    }
    let v = u64::from_le_bytes(bytes[*cur..*cur + 8].try_into().unwrap());
    *cur += 8;
    Ok(v)
}

fn read_exact_slice<'a>(
    bytes: &'a [u8],
    cur: &mut usize,
    len: usize,
    what: &str,
) -> Result<&'a [u8]> {
    let end = cur
        .checked_add(len)
        .ok_or_else(|| Error::SerdeError(format!("content store {what} length overflow")))?;
    if end > bytes.len() {
        return Err(Error::SerdeError(format!(
            "content store {what} truncated (len {len})"
        )));
    }
    let slice = &bytes[*cur..end];
    *cur = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn round_trip_save_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONTENT_STORE_FILE);
        let mut store = ContentStore::with_cache_file(path.clone());
        let hash = hash_text("large section body");
        store.insert_str(&hash, "large section body");
        store.save().unwrap();

        let loaded = ContentStore::load(path).unwrap();
        assert_eq!(loaded.get_str(&hash), Some("large section body"));
    }

    #[test]
    fn garbage_ascii_prefix_does_not_abort_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join(CONTENT_STORE_FILE);
        // Same 8-byte ASCII hex that was observed as a ~exabyte allocation request.
        std::fs::write(&path, b"d3be6c88").unwrap();
        let loaded = ContentStore::load(path).unwrap();
        assert!(loaded.is_empty());
    }
}
