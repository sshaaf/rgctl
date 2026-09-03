//! Heap-backed `HashMap` that avoids allocating until first insert.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::OnceLock;

fn empty_string_map() -> &'static HashMap<String, String> {
    static EMPTY: OnceLock<HashMap<String, String>> = OnceLock::new();
    EMPTY.get_or_init(HashMap::new)
}

/// `HashMap<String, String>` with no heap until the first insert.
#[derive(Debug, Clone, Default)]
pub struct LazyStringMap(Option<Box<HashMap<String, String>>>);

impl LazyStringMap {
    /// Empty map with no heap allocation.
    pub fn new() -> Self {
        Self(None)
    }

    /// Whether heap has been allocated.
    pub fn is_allocated(&self) -> bool {
        self.0.is_some()
    }

    /// Borrow iterator over entries.
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, String> {
        self.deref().iter()
    }

    /// Clone into a standard `HashMap` (for extension blobs / legacy APIs).
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        self.deref().clone()
    }

    /// Build from a populated map (allocates only when non-empty).
    pub fn from_hashmap(map: HashMap<String, String>) -> Self {
        if map.is_empty() {
            Self(None)
        } else {
            Self(Some(Box::new(map)))
        }
    }
}

impl IntoIterator for LazyStringMap {
    type Item = (String, String);
    type IntoIter = std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_hashmap().into_iter()
    }
}

impl<'a> IntoIterator for &'a LazyStringMap {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Deref for LazyStringMap {
    type Target = HashMap<String, String>;

    fn deref(&self) -> &Self::Target {
        self.0.as_deref().unwrap_or(empty_string_map())
    }
}

impl DerefMut for LazyStringMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.get_or_insert_with(Box::default)
    }
}

impl Serialize for LazyStringMap {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.deref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for LazyStringMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        if map.is_empty() {
            Ok(Self(None))
        } else {
            Ok(Self(Some(Box::new(map))))
        }
    }
}
