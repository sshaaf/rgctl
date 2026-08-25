//! String interning for memory-efficient graph storage
//!
//! Task 5.2.2: Deduplicate repeated strings across nodes

use crate::schema::SharedStr;
use rgctl_error::{Error, Result};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Deduplicates strings to reduce memory usage for large graphs.
#[derive(Debug, Default, Clone)]
pub struct StringInterner {
    pool: Arc<RwLock<HashSet<Arc<str>>>>,
    index: Arc<RwLock<HashMap<String, Arc<str>>>>,
}

impl StringInterner {
    /// Create a new empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a string, returning a shared handle.
    pub fn intern(&self, value: &str) -> Result<Arc<str>> {
        if let Ok(read) = self.index.read() {
            if let Some(existing) = read.get(value) {
                return Ok(existing.clone());
            }
        }

        let arc: Arc<str> = Arc::from(value);
        let mut write = self
            .pool
            .write()
            .map_err(|e| Error::GraphError(format!("StringInterner lock poisoned: {e}")))?;
        write.insert(Arc::clone(&arc));
        drop(write);

        self.index
            .write()
            .map_err(|e| Error::GraphError(format!("StringInterner lock poisoned: {e}")))?
            .insert(value.to_string(), arc.clone());
        Ok(arc)
    }

    /// Canonicalize in-place string storage using the intern pool.
    pub fn intern_string(&self, value: &mut String) {
        if let Ok(arc) = self.intern(value) {
            if value.as_str() != arc.as_ref() {
                *value = arc.as_ref().to_string();
            }
        }
    }

    /// Canonicalize a shared string handle using the intern pool.
    pub fn intern_shared(&self, value: &mut SharedStr) {
        if let Ok(arc) = self.intern(value.as_str()) {
            let next = SharedStr::from(arc);
            if value != &next {
                *value = next;
            }
        }
    }

    /// Number of unique interned strings.
    pub fn len(&self) -> usize {
        self.pool.read().map(|p| p.len()).unwrap_or(0)
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_deduplicates() {
        let interner = StringInterner::new();
        let a = interner.intern("hello").unwrap();
        let b = interner.intern("hello").unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(interner.len(), 1);
    }
}
