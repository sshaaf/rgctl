//! String interning for memory-efficient graph storage
//!
//! Task 5.2.2: Deduplicate repeated strings across nodes

use crate::schema::SharedStr;
use rgctl_error::{Error, Result};
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Deduplicates strings to reduce memory usage for large graphs.
#[derive(Debug, Default, Clone)]
pub struct StringInterner {
    pool: Arc<RwLock<HashSet<Arc<str>>>>,
}

impl StringInterner {
    /// Create a new empty interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a string, returning a shared handle.
    pub fn intern(&self, value: &str) -> Result<Arc<str>> {
        if let Ok(read) = self.pool.read() {
            if let Some(existing) = read.get(value) {
                return Ok(Arc::clone(existing));
            }
        }

        let arc: Arc<str> = Arc::from(value);
        let mut write = self
            .pool
            .write()
            .map_err(|e| Error::GraphError(format!("StringInterner lock poisoned: {e}")))?;
        if let Some(existing) = write.get(value) {
            return Ok(Arc::clone(existing));
        }
        write.insert(Arc::clone(&arc));
        Ok(arc)
    }

    /// Canonicalize in-place string storage using the intern pool.
    pub fn intern_string(&self, value: &mut String) -> Result<()> {
        let arc = self.intern(value)?;
        *value = arc.to_string();
        Ok(())
    }

    /// Canonicalize a shared string handle.
    pub fn intern_shared(&self, value: &mut SharedStr) -> Result<()> {
        let arc = self.intern(value)?;
        *value = SharedStr::from(arc);
        Ok(())
    }

    /// Number of unique interned strings.
    pub fn len(&self) -> usize {
        self.pool
            .read()
            .map(|pool| pool.len())
            .unwrap_or_default()
    }

    /// Whether the interner is empty.
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
