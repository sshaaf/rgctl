//! Serialized SCC blast-radius engine for mmap reload without recomputation.
//!
//! Format v2 stores sparse, zstd-compressed reachability rows. Trivial rows (only
//! the SCC itself reachable) are omitted and reconstructed on load. At query time
//! only the requested SCC row is decompressed ([`ReachabilityStore::row_bitset`]).

use bit_set::BitSet;
use memmap2::Mmap;
use rgctl_error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

use crate::blast_radius_scc::BlastRadiusEngine;

/// Magic bytes for blast engine snapshots (`RBSE`).
pub const BLAST_SNAPSHOT_MAGIC: [u8; 4] = *b"RBSE";
/// Legacy dense reachability format.
pub const BLAST_SNAPSHOT_VERSION_V1: u32 = 1;
/// Sparse + zstd-compressed reachability rows.
pub const BLAST_SNAPSHOT_VERSION: u32 = 2;
/// Default blast engine snapshot filename under `.rgctl/`.
pub const BLAST_SNAPSHOT_FILE: &str = "blast_engine.snapshot.bin";

/// One sparse reachability row (v2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReachabilityRow {
    /// SCC index for this row.
    pub scc_idx: u32,
    /// zstd-compressed little-endian `u64` bitset words.
    pub compressed: Vec<u8>,
}

/// Serializable blast-radius engine state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastEngineSnapshot {
    /// Digest of the graph snapshot this engine was built from.
    pub graph_digest: String,
    /// Number of strongly connected components in the call graph.
    pub scc_count: usize,
    /// SCC DAG edges (from_idx, to_idx).
    pub dag_edges: Vec<(usize, usize)>,
    /// Member UUIDs per SCC.
    pub scc_members: Vec<Vec<Uuid>>,
    /// Display name per SCC.
    pub scc_names: Vec<String>,
    /// Function UUID → SCC index.
    pub node_to_scc: Vec<(Uuid, usize)>,
    /// v1 dense reachability (legacy).
    #[serde(default)]
    pub reachability_words: Vec<Vec<u64>>,
    /// v2 sparse compressed reachability rows.
    #[serde(default)]
    pub reachability_rows: Vec<ReachabilityRow>,
}

impl BlastEngineSnapshot {
    /// Write snapshot file with v2 header.
    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let payload = bincode::serialize(self).map_err(serde_err)?;
        let mut file = File::create(path)?;
        file.write_all(&BLAST_SNAPSHOT_MAGIC)?;
        file.write_all(&BLAST_SNAPSHOT_VERSION.to_le_bytes())?;
        file.write_all(&(payload.len() as u64).to_le_bytes())?;
        file.write_all(&payload)?;
        Ok(())
    }

    /// Read the embedded graph digest without deserializing reachability rows.
    pub fn read_graph_digest(path: &Path) -> Result<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }
        let file = File::open(path)?;
        // SAFETY: read-only map for parsing the digest prefix.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(parse_graph_digest_from_mmap(&mmap))
    }

    /// Returns true when an on-disk snapshot matches the expected graph digest.
    pub fn digest_matches(path: &Path, expected: &str) -> Result<bool> {
        Ok(Self::read_graph_digest(path)?.as_deref() == Some(expected))
    }

    /// Load snapshot from disk via mmap (avoids an extra full-file buffer copy).
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        // SAFETY: Read-only map of a file we own for the snapshot lifetime; parse only reads bytes.
        let mmap = unsafe { Mmap::map(&file)? };
        parse_blast_payload(&mmap)
    }

    /// Default path under a repository root.
    pub fn default_path(repo_root: &Path) -> PathBuf {
        repo_root
            .join(rgctl_graph::code_graph::GRAPH_DIR)
            .join(BLAST_SNAPSHOT_FILE)
    }
}

pub(crate) fn bitset_to_words(bs: &BitSet, bit_len: usize) -> Vec<u64> {
    let word_count = bit_len.div_ceil(64);
    let mut words = vec![0u64; word_count];
    for idx in bs.iter() {
        if idx >= bit_len {
            break;
        }
        words[idx / 64] |= 1u64 << (idx % 64);
    }
    words
}

pub(crate) fn words_popcount(words: &[u64]) -> u32 {
    words.iter().map(|w| w.count_ones()).sum()
}

pub(crate) fn compress_words(words: &[u64]) -> Result<Vec<u8>> {
    let raw: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    zstd::encode_all(raw.as_slice(), 3)
        .map_err(|e| Error::SerdeError(format!("zstd compress: {e}")))
}

pub(crate) fn decompress_words(compressed: &[u8], word_count: usize) -> Result<Vec<u64>> {
    let raw = zstd::decode_all(compressed)
        .map_err(|e| Error::SerdeError(format!("zstd decompress: {e}")))?;
    if raw.len() != word_count * 8 {
        return Err(Error::SerdeError(format!(
            "reachability row size mismatch: expected {} bytes, got {}",
            word_count * 8,
            raw.len()
        )));
    }
    Ok(raw
        .chunks_exact(8)
        .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
        .collect())
}

pub(crate) fn words_to_bitset(words: &[u64], bit_len: usize) -> BitSet {
    let mut bs = BitSet::new();
    for (w, &word) in words.iter().enumerate() {
        for b in 0..64 {
            let idx = w * 64 + b;
            if idx >= bit_len {
                break;
            }
            if (word & (1u64 << b)) != 0 {
                bs.insert(idx);
            }
        }
    }
    bs
}

/// When `scc_count / node_count` exceeds this, skip eager bitset propagation.
pub const FLAT_SCC_COMPRESSION_THRESHOLD: f64 = 0.90;

/// Max cached on-demand reachability rows (bounds memory on flat graphs).
pub const ON_DEMAND_REACHABILITY_CACHE_CAP: usize = 4096;

/// Lazy or eager SCC reachability storage for [`BlastRadiusEngine`].
pub struct ReachabilityStore {
    scc_count: usize,
    word_count: usize,
    backing: ReachabilityBacking,
}

enum ReachabilityBacking {
    Eager(Vec<BitSet>),
    Lazy {
        rows: HashMap<u32, Vec<u8>>,
        cache: Mutex<ReachabilityCache>,
    },
    DagOnDemand {
        incoming: Vec<Vec<usize>>,
        cache: Mutex<ReachabilityCache>,
    },
}

#[derive(Debug)]
struct ReachabilityCache {
    map: HashMap<usize, BitSet>,
    order: VecDeque<usize>,
    cap: usize,
}

impl ReachabilityCache {
    fn new(cap: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            cap,
        }
    }

    fn get(&self, key: usize) -> Option<BitSet> {
        self.map.get(&key).cloned()
    }

    fn insert(&mut self, key: usize, value: BitSet) {
        if let Some(pos) = self.order.iter().position(|&k| k == key) {
            self.order.remove(pos);
        } else if self.map.len() >= self.cap {
            if let Some(evicted) = self.order.pop_front() {
                self.map.remove(&evicted);
            }
        }
        self.order.push_back(key);
        self.map.insert(key, value);
    }
}

/// Incoming SCC adjacency (caller lists) from condensed DAG edges.
pub(crate) fn incoming_from_dag_edges(
    dag_edges: &[(usize, usize)],
    scc_count: usize,
) -> Vec<Vec<usize>> {
    let mut incoming = vec![Vec::new(); scc_count];
    for &(from, to) in dag_edges {
        if to < scc_count && from < scc_count {
            incoming[to].push(from);
        }
    }
    incoming
}

fn reachability_row_on_demand(incoming: &[Vec<usize>], scc_id: usize, scc_count: usize) -> BitSet {
    let mut reachable = BitSet::new();
    let mut stack = vec![scc_id];
    reachable.insert(scc_id);
    while let Some(cur) = stack.pop() {
        for &parent in incoming.get(cur).map(Vec::as_slice).unwrap_or(&[]) {
            if parent < scc_count && reachable.insert(parent) {
                stack.push(parent);
            }
        }
    }
    reachable
}

impl ReachabilityStore {
    /// Build an eager in-memory store from pre-expanded SCC rows.
    pub fn from_eager(reachability: Vec<BitSet>, scc_count: usize) -> Self {
        Self {
            scc_count,
            word_count: scc_count.div_ceil(64),
            backing: ReachabilityBacking::Eager(reachability),
        }
    }

    /// Query-time reachability from a condensed SCC DAG (no eager bitset matrix).
    pub fn from_dag_on_demand(incoming: Vec<Vec<usize>>, scc_count: usize) -> Self {
        Self {
            scc_count,
            word_count: scc_count.div_ceil(64),
            backing: ReachabilityBacking::DagOnDemand {
                incoming,
                cache: Mutex::new(ReachabilityCache::new(ON_DEMAND_REACHABILITY_CACHE_CAP)),
            },
        }
    }

    /// Load v2 snapshots lazily; expand v1 dense snapshots eagerly; fall back to DAG on-demand.
    pub fn from_snapshot(snapshot: &BlastEngineSnapshot) -> Result<Self> {
        let scc_count = snapshot.scc_count;
        if !snapshot.reachability_rows.is_empty() {
            let rows = snapshot
                .reachability_rows
                .iter()
                .map(|row| (row.scc_idx, row.compressed.clone()))
                .collect();
            return Ok(Self {
                scc_count,
                word_count: scc_count.div_ceil(64),
                backing: ReachabilityBacking::Lazy {
                    rows,
                    cache: Mutex::new(ReachabilityCache::new(ON_DEMAND_REACHABILITY_CACHE_CAP)),
                },
            });
        }
        if !snapshot.reachability_words.is_empty() {
            return Ok(Self::from_eager(
                reachability_from_snapshot_eager_only(snapshot)?,
                scc_count,
            ));
        }
        let incoming = incoming_from_dag_edges(&snapshot.dag_edges, scc_count);
        Ok(Self::from_dag_on_demand(incoming, scc_count))
    }

    /// Number of SCCs in the condensed call graph.
    pub fn scc_count(&self) -> usize {
        self.scc_count
    }

    /// True when rows are not fully materialized in memory (compressed snapshot or DAG on-demand).
    pub fn is_lazy(&self) -> bool {
        !matches!(self.backing, ReachabilityBacking::Eager(_))
    }

    /// True when reachability is computed per query via DAG traversal (flat graphs).
    pub fn is_on_demand(&self) -> bool {
        matches!(self.backing, ReachabilityBacking::DagOnDemand { .. })
    }

    /// Full eager row slice when the store is not lazy.
    pub fn eager_slice(&self) -> Option<&[BitSet]> {
        match &self.backing {
            ReachabilityBacking::Eager(v) => Some(v.as_slice()),
            ReachabilityBacking::Lazy { .. } | ReachabilityBacking::DagOnDemand { .. } => None,
        }
    }

    /// Reachability bitset for one SCC (self-only when no sparse row exists).
    pub fn row_bitset(&self, scc_id: usize) -> Result<BitSet> {
        if scc_id >= self.scc_count {
            return Err(Error::SerdeError(format!(
                "reachability row index {scc_id} out of range (scc_count={})",
                self.scc_count
            )));
        }
        match &self.backing {
            ReachabilityBacking::Eager(v) => Ok(v[scc_id].clone()),
            ReachabilityBacking::Lazy { rows, cache } => {
                let mut cache = cache.lock().expect("reachability cache lock");
                if let Some(cached) = cache.get(scc_id) {
                    return Ok(cached);
                }
                let mut bs = BitSet::new();
                bs.insert(scc_id);
                if let Some(compressed) = rows.get(&(scc_id as u32)) {
                    let words = decompress_words(compressed, self.word_count)?;
                    bs = words_to_bitset(&words, self.scc_count);
                }
                cache.insert(scc_id, bs.clone());
                Ok(bs)
            }
            ReachabilityBacking::DagOnDemand { incoming, cache } => {
                let mut cache = cache.lock().expect("reachability cache lock");
                if let Some(cached) = cache.get(scc_id) {
                    return Ok(cached);
                }
                let bs = reachability_row_on_demand(incoming, scc_id, self.scc_count);
                cache.insert(scc_id, bs.clone());
                Ok(bs)
            }
        }
    }
}

fn reachability_from_snapshot_eager_only(snapshot: &BlastEngineSnapshot) -> Result<Vec<BitSet>> {
    let scc_count = snapshot.scc_count;
    let mut reachability: Vec<BitSet> = (0..scc_count)
        .map(|idx| {
            let mut bs = BitSet::new();
            bs.insert(idx);
            bs
        })
        .collect();

    if snapshot.reachability_words.is_empty() {
        return Ok(reachability);
    }
    if snapshot.reachability_words.len() != scc_count {
        return Err(Error::SerdeError(format!(
            "v1 reachability row count {} != scc_count {scc_count}",
            snapshot.reachability_words.len()
        )));
    }
    for (idx, words) in snapshot.reachability_words.iter().enumerate() {
        reachability[idx] = words_to_bitset(words, scc_count);
    }
    Ok(reachability)
}

fn parse_graph_digest_from_mmap(mmap: &[u8]) -> Option<String> {
    if mmap.len() < 16 || mmap[0..4] != BLAST_SNAPSHOT_MAGIC {
        return None;
    }
    let version = u32::from_le_bytes(mmap[4..8].try_into().ok()?);
    if version != BLAST_SNAPSHOT_VERSION && version != BLAST_SNAPSHOT_VERSION_V1 {
        return None;
    }
    let payload_len = u64::from_le_bytes(mmap[8..16].try_into().ok()?) as usize;
    let payload = mmap.get(16..16usize.checked_add(payload_len)?)?;
    parse_bincode_leading_string(payload)
}

/// First field of [`BlastEngineSnapshot`] is `graph_digest: String` (bincode u64 len + utf8).
fn parse_bincode_leading_string(payload: &[u8]) -> Option<String> {
    if payload.len() < 8 {
        return None;
    }
    let len = u64::from_le_bytes(payload[0..8].try_into().ok()?) as usize;
    let end = 8usize.checked_add(len)?;
    let bytes = payload.get(8..end)?;
    std::str::from_utf8(bytes).ok().map(str::to_string)
}

fn parse_blast_payload(bytes: &[u8]) -> Result<BlastEngineSnapshot> {
    if bytes.len() < 16 {
        return Err(Error::SerdeError("blast snapshot truncated".into()));
    }
    if bytes[0..4] != BLAST_SNAPSHOT_MAGIC {
        return Err(Error::SerdeError("invalid blast snapshot magic".into()));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != BLAST_SNAPSHOT_VERSION && version != BLAST_SNAPSHOT_VERSION_V1 {
        return Err(Error::SerdeError(format!(
            "unsupported blast snapshot version {version}"
        )));
    }
    let payload_len = u64::from_le_bytes(bytes[8..16].try_into().unwrap()) as usize;
    if bytes.len() < 16 + payload_len {
        return Err(Error::SerdeError("blast snapshot payload truncated".into()));
    }
    bincode::deserialize(&bytes[16..16 + payload_len]).map_err(serde_err)
}

fn serde_err(e: bincode::Error) -> Error {
    Error::SerdeError(format!("blast snapshot: {e}"))
}

/// Load engine from disk if digest matches.
pub fn try_load_engine(repo_root: &Path, graph_digest: &str) -> Result<Option<BlastRadiusEngine>> {
    let path = BlastEngineSnapshot::default_path(repo_root);
    if !path.exists() {
        return Ok(None);
    }
    let snap = BlastEngineSnapshot::load_from_path(&path)?;
    if snap.graph_digest != graph_digest {
        return Ok(None);
    }
    BlastRadiusEngine::from_engine_snapshot(snap).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_graph_digest_without_full_deserialize() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("blast_engine.snapshot.bin");
        let snap = BlastEngineSnapshot {
            graph_digest: "abc123digest".into(),
            scc_count: 2,
            dag_edges: vec![],
            scc_members: vec![vec![], vec![]],
            scc_names: vec!["a".into(), "b".into()],
            node_to_scc: vec![],
            reachability_words: vec![],
            reachability_rows: vec![],
        };
        snap.write_to_path(&path).unwrap();
        assert_eq!(
            BlastEngineSnapshot::read_graph_digest(&path)
                .unwrap()
                .as_deref(),
            Some("abc123digest")
        );
        assert!(BlastEngineSnapshot::digest_matches(&path, "abc123digest").unwrap());
        assert!(!BlastEngineSnapshot::digest_matches(&path, "other").unwrap());
    }

    #[test]
    fn compress_round_trip() {
        let words = vec![0b1010u64, 0u64, 1u64 << 63];
        let compressed = compress_words(&words).unwrap();
        let restored = decompress_words(&compressed, words.len()).unwrap();
        assert_eq!(words, restored);
    }

    #[test]
    fn sparse_self_only_rows_omitted() {
        let scc_count = 4;
        let mut reachability: Vec<BitSet> = (0..scc_count)
            .map(|idx| {
                let mut bs = BitSet::new();
                bs.insert(idx);
                bs
            })
            .collect();
        reachability[2].insert(1);

        let mut rows = Vec::new();
        for (idx, bs) in reachability.iter().enumerate() {
            let words = bitset_to_words(bs, scc_count);
            if words_popcount(&words) <= 1 {
                continue;
            }
            rows.push(ReachabilityRow {
                scc_idx: idx as u32,
                compressed: compress_words(&words).unwrap(),
            });
        }
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].scc_idx, 2);

        let snap = BlastEngineSnapshot {
            graph_digest: "test".into(),
            scc_count,
            dag_edges: vec![],
            scc_members: vec![vec![]; scc_count],
            scc_names: vec!["a".into(); scc_count],
            node_to_scc: vec![],
            reachability_words: vec![],
            reachability_rows: rows,
        };
        let store = ReachabilityStore::from_snapshot(&snap).unwrap();
        let loaded: Vec<BitSet> = (0..store.scc_count())
            .map(|idx| store.row_bitset(idx).unwrap())
            .collect();
        assert_eq!(loaded[2].contains(1), reachability[2].contains(1));
        assert_eq!(loaded[0].contains(0), true);
    }

    #[test]
    fn on_demand_reachability_matches_eager_propagation() {
        let scc_count = 5;
        let dag_edges = vec![(0, 1), (1, 2), (2, 3), (1, 4)];
        let incoming = incoming_from_dag_edges(&dag_edges, scc_count);

        let mut eager: Vec<BitSet> = vec![BitSet::new(); scc_count];
        let mut dag: petgraph::graph::DiGraph<(), ()> = petgraph::graph::DiGraph::new();
        for _ in 0..scc_count {
            dag.add_node(());
        }
        for (from, to) in &dag_edges {
            dag.add_edge(
                petgraph::graph::NodeIndex::new(*from),
                petgraph::graph::NodeIndex::new(*to),
                (),
            );
        }
        let sorted = petgraph::algo::toposort(&dag, None).unwrap();
        for &scc_idx in sorted.iter() {
            let scc_id = scc_idx.index();
            let mut reach = BitSet::new();
            reach.insert(scc_id);
            for parent_idx in dag.neighbors_directed(scc_idx, petgraph::Direction::Incoming) {
                reach.union_with(&eager[parent_idx.index()]);
            }
            eager[scc_id] = reach;
        }

        let on_demand = ReachabilityStore::from_dag_on_demand(incoming, scc_count);
        for idx in 0..scc_count {
            let eager_row = &eager[idx];
            let lazy_row = on_demand.row_bitset(idx).unwrap();
            assert_eq!(
                eager_row.iter().collect::<Vec<_>>(),
                lazy_row.iter().collect::<Vec<_>>(),
                "row {idx}"
            );
        }
    }

    #[test]
    fn lazy_store_defers_row_expand_until_query() {
        let scc_count = 4;
        let mut reachability: Vec<BitSet> = (0..scc_count)
            .map(|idx| {
                let mut bs = BitSet::new();
                bs.insert(idx);
                bs
            })
            .collect();
        reachability[2].insert(1);

        let mut rows = Vec::new();
        for (idx, bs) in reachability.iter().enumerate() {
            let words = bitset_to_words(bs, scc_count);
            if words_popcount(&words) <= 1 {
                continue;
            }
            rows.push(ReachabilityRow {
                scc_idx: idx as u32,
                compressed: compress_words(&words).unwrap(),
            });
        }

        let snap = BlastEngineSnapshot {
            graph_digest: "lazy".into(),
            scc_count,
            dag_edges: vec![],
            scc_members: vec![vec![]; scc_count],
            scc_names: vec!["a".into(); scc_count],
            node_to_scc: vec![],
            reachability_words: vec![],
            reachability_rows: rows,
        };

        let store = ReachabilityStore::from_snapshot(&snap).unwrap();
        assert!(store.is_lazy());
        assert!(!store.is_on_demand());
        let row = store.row_bitset(2).unwrap();
        assert!(row.contains(1));
    }

    #[test]
    fn snapshot_without_rows_uses_dag_on_demand() {
        let snap = BlastEngineSnapshot {
            graph_digest: "flat".into(),
            scc_count: 3,
            dag_edges: vec![(0, 1), (1, 2)],
            scc_members: vec![vec![], vec![], vec![]],
            scc_names: vec!["a".into(), "b".into(), "c".into()],
            node_to_scc: vec![],
            reachability_words: vec![],
            reachability_rows: vec![],
        };

        let store = ReachabilityStore::from_snapshot(&snap).unwrap();
        assert!(store.is_on_demand());
        let row = store.row_bitset(2).unwrap();
        assert!(row.contains(2));
        assert!(row.contains(1));
        assert!(row.contains(0));
    }
}
