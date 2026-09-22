//! Heap-backed `HashMap` that avoids allocating until first insert.

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, HashMap};
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

    /// Clone into a sorted `BTreeMap` for deterministic snapshot serialization.
    pub fn to_btreemap(&self) -> BTreeMap<String, String> {
        self.deref().iter().map(|(k, v)| (k.clone(), v.clone())).collect()
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

impl FromIterator<(String, String)> for LazyStringMap {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self::from_hashmap(iter.into_iter().collect())
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
        let map = self.deref();
        let mut seq = serializer.serialize_map(Some(map.len()))?;
        if !map.is_empty() {
            let mut entries: Vec<(&String, &String)> = map.iter().collect();
            entries.sort_unstable_by_key(|&(k, _)| k);
            for (k, v) in entries {
                seq.serialize_entry(k, v)?;
            }
        }
        seq.end()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lazy_string_map_serialization_is_order_independent() {
        let mut m1 = LazyStringMap::new();
        m1.insert("cyclomatic".into(), "1".into());
        m1.insert("cognitive".into(), "2".into());
        m1.insert("loc".into(), "3".into());
        m1.insert("nesting_depth".into(), "4".into());

        let mut m2 = LazyStringMap::new();
        m2.insert("nesting_depth".into(), "4".into());
        m2.insert("loc".into(), "3".into());
        m2.insert("cognitive".into(), "2".into());
        m2.insert("cyclomatic".into(), "1".into());

        let b1 = bincode::serialize(&m1).expect("serialize m1");
        let b2 = bincode::serialize(&m2).expect("serialize m2");
        assert_eq!(
            b1, b2,
            "LazyStringMap bincode output must be identical regardless of insertion order"
        );
    }

    #[test]
    fn lazy_string_map_empty_and_cleared_equivalence() {
        let m_unallocated = LazyStringMap::new();
        let mut m_cleared = LazyStringMap::new();
        m_cleared.insert("k".into(), "v".into());
        m_cleared.clear();

        let b1 = bincode::serialize(&m_unallocated).unwrap();
        let b2 = bincode::serialize(&m_cleared).unwrap();
        assert_eq!(
            b1, b2,
            "Unallocated and cleared maps must serialize to identical bytes"
        );
    }

    #[test]
    fn lazy_string_map_special_characters_and_prefix_collisions() {
        let mut m1 = LazyStringMap::new();
        m1.insert("konveyor.io/target".into(), "quarkus".into());
        m1.insert("konveyor.io/target-version".into(), "3.0".into());
        m1.insert("loc".into(), "10".into());
        m1.insert("loc_lines".into(), "10".into());
        m1.insert("loc:lines".into(), "10".into());
        m1.insert("Loc".into(), "uppercase".into());
        m1.insert("".into(), "empty_key".into());
        m1.insert("empty_val".into(), "".into());

        let mut m2 = LazyStringMap::new();
        m2.insert("empty_val".into(), "".into());
        m2.insert("".into(), "empty_key".into());
        m2.insert("Loc".into(), "uppercase".into());
        m2.insert("loc:lines".into(), "10".into());
        m2.insert("loc_lines".into(), "10".into());
        m2.insert("loc".into(), "10".into());
        m2.insert("konveyor.io/target-version".into(), "3.0".into());
        m2.insert("konveyor.io/target".into(), "quarkus".into());

        let b1 = bincode::serialize(&m1).unwrap();
        let b2 = bincode::serialize(&m2).unwrap();
        assert_eq!(
            b1, b2,
            "Must serialize identically with prefix collisions and special chars"
        );
    }
}
