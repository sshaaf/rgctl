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
            let blobs: HashMap<String, Vec<u8>> = bincode::deserialize(&bytes)
                .map_err(|e| Error::SerdeError(format!("content store decode: {e}")))?;
            Ok(Self {
                blobs,
                cache_file: Some(cache_file),
            })
        } else {
            Ok(Self::with_cache_file(cache_file))
        }
    }

    /// Default path under a repository root.
    pub fn default_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".rgctl").join(CONTENT_STORE_FILE)
    }
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
}
