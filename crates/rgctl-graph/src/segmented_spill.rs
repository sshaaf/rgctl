//! Append-only disk spill for extract → columnar compile without full `Vec` residency.
//!
//! Record layout (little-endian):
//! - **nodes.seg**: `id[16]` + `len[u64]` + `bincode(Node)`
//! - **edges.seg**: `from[16]` + `to[16]` + `edge_type[u8]` + `pad[7]` + `len[u64]` + `bincode(Edge)`
//!
//! Compile externally sorts by the same keys as [`crate::write_columnar_from_nodes_edges`]
//! and hashes the spilled bincode blobs for digest identity.

use crate::columnar_snapshot::{
    EdgeRow, StringPool, append_node_columnar_prehashed, write_columnar_assembled,
};
use crate::csr::edge_type_to_u8;
use crate::schema::{Edge, Node, NodeType};
use rgctl_error::{Error, Result};
use std::cmp::Ordering;
use rayon::prelude::*;
use std::thread;
use std::collections::{BinaryHeap, HashMap};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Default run size for external merge-sort (~64 MiB of record payload).
pub const DEFAULT_SORT_RUN_BYTES: usize = 64 * 1024 * 1024;

const NODE_KEY_LEN: usize = 16;
const EDGE_KEY_LEN: usize = 16 + 16 + 8; // from + to + type/pad

/// Append-only spill writers for nodes and edges during extract.
pub struct SegmentedSpill {
    dir: PathBuf,
    nodes: BufWriter<File>,
    edges: BufWriter<File>,
    node_count: usize,
    edge_count: usize,
}

/// Closed spill ready for external sort + columnar compile.
pub struct FinishedSpill {
    dir: PathBuf,
    node_count: usize,
    edge_count: usize,
}

impl SegmentedSpill {
    /// Create spill files under `dir` (created if missing).
    pub fn create(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;
        let nodes = BufWriter::with_capacity(8 * 1024 * 1024, File::create(dir.join("nodes.seg"))?);
        let edges = BufWriter::with_capacity(8 * 1024 * 1024, File::create(dir.join("edges.seg"))?);
        Ok(Self {
            dir,
            nodes,
            edges,
            node_count: 0,
            edge_count: 0,
        })
    }

    /// Spill directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Nodes appended so far.
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Edges appended so far.
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    /// Append a node as length-prefixed bincode with UUID key prefix.
    pub fn append_node(&mut self, node: &Node) -> Result<()> {
        let blob = bincode::serialize(node)
            .map_err(|e| Error::SerdeError(format!("segmented spill node serialize: {e}")))?;
        self.nodes.write_all(node.id.as_bytes())?;
        self.nodes.write_all(&(blob.len() as u64).to_le_bytes())?;
        self.nodes.write_all(&blob)?;
        self.node_count += 1;
        Ok(())
    }

    /// Append an edge as length-prefixed bincode with sort key prefix.
    ///
    /// Serializes [`Edge::for_columnar_digest`] so digest bytes match topology-only
    /// columnar rows after rematerialize/compact.
    pub fn append_edge(&mut self, edge: &Edge) -> Result<()> {
        let canonical = edge.for_columnar_digest();
        let blob = bincode::serialize(&canonical)
            .map_err(|e| Error::SerdeError(format!("segmented spill edge serialize: {e}")))?;
        let mut key = [0u8; EDGE_KEY_LEN];
        key[..16].copy_from_slice(canonical.from.as_bytes());
        key[16..32].copy_from_slice(canonical.to.as_bytes());
        key[32] = edge_type_to_u8(canonical.edge_type);
        self.edges.write_all(&key)?;
        self.edges.write_all(&(blob.len() as u64).to_le_bytes())?;
        self.edges.write_all(&blob)?;
        self.edge_count += 1;
        Ok(())
    }

    /// Flush and close writers.
    pub fn finish(mut self) -> Result<FinishedSpill> {
        self.nodes.flush()?;
        self.edges.flush()?;
        // Drop writers so files are closed before sort reopens them.
        drop(self.nodes);
        drop(self.edges);
        Ok(FinishedSpill {
            dir: self.dir,
            node_count: self.node_count,
            edge_count: self.edge_count,
        })
    }
}

impl FinishedSpill {
    /// Spill directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Nodes appended so far.
    pub fn node_count(&self) -> usize {
        self.node_count
    }

    /// Edges appended so far.
    pub fn edge_count(&self) -> usize {
        self.edge_count
    }

    /// Remove the spill directory tree.
    pub fn cleanup(self) -> Result<()> {
        if self.dir.exists() {
            fs::remove_dir_all(&self.dir)?;
        }
        Ok(())
    }
}

/// Read externally-sorted nodes and edges from a finished spill (for delta compact).
pub fn materialize_sorted_graph(spill: &FinishedSpill) -> Result<(Vec<Node>, Vec<Edge>)> {
    let dir = &spill.dir;
    let nodes_unsorted = dir.join("nodes.seg");
    let edges_unsorted = dir.join("edges.seg");
    let nodes_sorted = dir.join("nodes.sorted.seg");
    let edges_sorted = dir.join("edges.sorted.seg");

    thread::scope(|scope| -> Result<()> {
        let node_err = scope.spawn(|| {
            external_sort_records(
                &nodes_unsorted,
                &nodes_sorted,
                NODE_KEY_LEN,
                DEFAULT_SORT_RUN_BYTES,
                spill.node_count,
            )
        });
        let edge_err = scope.spawn(|| {
            external_sort_records(
                &edges_unsorted,
                &edges_sorted,
                EDGE_KEY_LEN,
                DEFAULT_SORT_RUN_BYTES,
                spill.edge_count,
            )
        });
        node_err.join().map_err(|_| Error::GraphError("node sort panicked".into()))??;
        edge_err.join().map_err(|_| Error::GraphError("edge sort panicked".into()))??;
        Ok(())
    })?;

    let mut nodes = Vec::with_capacity(spill.node_count);
    {
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(&nodes_sorted)?);
        for _ in 0..spill.node_count {
            let mut key = [0u8; NODE_KEY_LEN];
            reader.read_exact(&mut key)?;
            let mut len_buf = [0u8; 8];
            reader.read_exact(&mut len_buf)?;
            let len = u64::from_le_bytes(len_buf) as usize;
            let mut blob = vec![0u8; len];
            reader.read_exact(&mut blob)?;
            let node: Node = bincode::deserialize(&blob)
                .map_err(|e| Error::SerdeError(format!("segmented spill node deserialize: {e}")))?;
            nodes.push(node);
        }
    }

    let mut edges = Vec::with_capacity(spill.edge_count);
    {
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(&edges_sorted)?);
        for _ in 0..spill.edge_count {
            let mut key = [0u8; EDGE_KEY_LEN];
            reader.read_exact(&mut key)?;
            let mut len_buf = [0u8; 8];
            reader.read_exact(&mut len_buf)?;
            let len = u64::from_le_bytes(len_buf) as usize;
            let mut blob = vec![0u8; len];
            reader.read_exact(&mut blob)?;
            let edge: Edge = bincode::deserialize(&blob)
                .map_err(|e| Error::SerdeError(format!("segmented spill edge deserialize: {e}")))?;
            edges.push(edge);
        }
    }

    Ok((nodes, edges))
}

/// Compile a columnar v2 snapshot from a finished spill (external sort + stream encode).
///
/// Digest matches [`crate::write_columnar_from_nodes_edges`] for the same node/edge set.
/// Removes the spill directory on success.
pub fn write_columnar_from_spill(spill: FinishedSpill, path: &Path) -> Result<String> {
    let dir = spill.dir.clone();
    let node_count = spill.node_count;
    let edge_count = spill.edge_count;

    let nodes_unsorted = dir.join("nodes.seg");
    let edges_unsorted = dir.join("edges.seg");
    let nodes_sorted = dir.join("nodes.sorted.seg");
    let edges_sorted = dir.join("edges.sorted.seg");

    thread::scope(|scope| -> Result<()> {
        let node_err = scope.spawn(|| {
            external_sort_records(
                &nodes_unsorted,
                &nodes_sorted,
                NODE_KEY_LEN,
                DEFAULT_SORT_RUN_BYTES,
                node_count,
            )
        });
        let edge_err = scope.spawn(|| {
            external_sort_records(
                &edges_unsorted,
                &edges_sorted,
                EDGE_KEY_LEN,
                DEFAULT_SORT_RUN_BYTES,
                edge_count,
            )
        });
        node_err.join().map_err(|_| Error::GraphError("node sort panicked".into()))??;
        edge_err.join().map_err(|_| Error::GraphError("edge sort panicked".into()))??;
        Ok(())
    })?;

    let mut hasher = blake3::Hasher::new();
    let mut strings = StringPool::new();
    let mut node_rows = Vec::with_capacity(node_count);
    let mut extensions_blob = Vec::new();
    let mut name_index: HashMap<String, Vec<Uuid>> = HashMap::new();
    let mut type_index: HashMap<NodeType, Vec<Uuid>> = HashMap::new();

    {
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(&nodes_sorted)?);
        for _ in 0..node_count {
            let mut key = [0u8; NODE_KEY_LEN];
            reader.read_exact(&mut key)?;
            let mut len_buf = [0u8; 8];
            reader.read_exact(&mut len_buf)?;
            let len = u64::from_le_bytes(len_buf) as usize;
            let mut blob = vec![0u8; len];
            reader.read_exact(&mut blob)?;
            let node: Node = bincode::deserialize(&blob)
                .map_err(|e| Error::SerdeError(format!("segmented spill node deserialize: {e}")))?;
            append_node_columnar_prehashed(
                &node,
                &blob,
                &mut hasher,
                &mut strings,
                &mut extensions_blob,
                &mut name_index,
                &mut type_index,
                &mut node_rows,
                None,
            )?;
        }
    }

    let mut edge_rows = Vec::with_capacity(edge_count);
    {
        let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(&edges_sorted)?);
        for _ in 0..edge_count {
            let mut key = [0u8; EDGE_KEY_LEN];
            reader.read_exact(&mut key)?;
            let mut len_buf = [0u8; 8];
            reader.read_exact(&mut len_buf)?;
            let len = u64::from_le_bytes(len_buf) as usize;
            let mut blob = vec![0u8; len];
            reader.read_exact(&mut blob)?;
            hasher.update(&blob);
            let from = Uuid::from_bytes(key[..16].try_into().unwrap());
            let to = Uuid::from_bytes(key[16..32].try_into().unwrap());
            let edge_type = key[32];
            edge_rows.push(EdgeRow {
                from: *from.as_bytes(),
                to: *to.as_bytes(),
                edge_type,
                _pad: [0; 7],
            });
        }
    }

    let content_digest = hasher.finalize().to_hex().to_string();
    write_columnar_assembled(
        path,
        &node_rows,
        &edge_rows,
        &strings,
        &extensions_blob,
        &name_index,
        &type_index,
        &content_digest,
    )?;

    spill.cleanup()?;
    Ok(content_digest)
}

fn external_sort_records(
    input: &Path,
    output: &Path,
    key_len: usize,
    run_bytes: usize,
    record_count: usize,
) -> Result<()> {
    if record_count == 0 {
        File::create(output)?;
        return Ok(());
    }

    let meta = fs::metadata(input)?;
    // Small enough to sort in one pass in RAM.
    if meta.len() as usize <= run_bytes || record_count < 10_000 {
        let mut records = read_all_records(input, key_len, record_count)?;
        records.par_sort_unstable_by(|a, b| a.key[..a.key_len].cmp(&b.key[..b.key_len]));
        write_records(output, &records)?;
        return Ok(());
    }

    let parent = output.parent().unwrap_or_else(|| Path::new("."));
    let run_prefix = output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("run");
    let mut run_paths = Vec::new();
    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(input)?);
    let mut remaining = record_count;
    let mut run_idx = 0usize;

    while remaining > 0 {
        let mut batch = Vec::new();
        let mut batch_bytes = 0usize;
        while remaining > 0 && (batch.is_empty() || batch_bytes < run_bytes) {
            let rec = read_one_record(&mut reader, key_len)?;
            batch_bytes += rec.key_len + 8 + rec.blob.len();
            batch.push(rec);
            remaining -= 1;
        }
        batch.par_sort_unstable_by(|a, b| a.key[..a.key_len].cmp(&b.key[..b.key_len]));
        let run_path = parent.join(format!("{run_prefix}-{run_idx}.seg"));
        write_records(&run_path, &batch)?;
        run_paths.push(run_path);
        run_idx += 1;
    }

    if run_paths.len() == 1 {
        fs::rename(&run_paths[0], output)?;
        return Ok(());
    }

    k_way_merge(&run_paths, output, key_len)?;
    for p in run_paths {
        let _ = fs::remove_file(p);
    }
    Ok(())
}

const MAX_SPILL_KEY_LEN: usize = EDGE_KEY_LEN;

struct SpillRecord {
    key: [u8; MAX_SPILL_KEY_LEN],
    key_len: usize,
    blob: Vec<u8>,
}

fn read_one_record<R: Read>(reader: &mut R, key_len: usize) -> Result<SpillRecord> {
    let mut key = [0u8; MAX_SPILL_KEY_LEN];
    reader.read_exact(&mut key[..key_len])?;
    let mut len_buf = [0u8; 8];
    reader.read_exact(&mut len_buf)?;
    let len = u64::from_le_bytes(len_buf) as usize;
    let mut blob = vec![0u8; len];
    reader.read_exact(&mut blob)?;
    Ok(SpillRecord {
        key,
        key_len,
        blob,
    })
}

fn read_all_records(path: &Path, key_len: usize, count: usize) -> Result<Vec<SpillRecord>> {
    let mut reader = BufReader::with_capacity(8 * 1024 * 1024, File::open(path)?);
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_one_record(&mut reader, key_len)?);
    }
    Ok(out)
}

fn write_records(path: &Path, records: &[SpillRecord]) -> Result<()> {
    let mut w = BufWriter::with_capacity(8 * 1024 * 1024, File::create(path)?);
    for rec in records {
        w.write_all(&rec.key[..rec.key_len])?;
        w.write_all(&(rec.blob.len() as u64).to_le_bytes())?;
        w.write_all(&rec.blob)?;
    }
    w.flush()?;
    Ok(())
}

#[derive(Eq)]
struct HeapEntry {
    key: [u8; MAX_SPILL_KEY_LEN],
    key_len: usize,
    blob: Vec<u8>,
    run_idx: usize,
}

impl PartialEq for HeapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key[..self.key_len] == other.key[..other.key_len] && self.run_idx == other.run_idx
    }
}

impl Ord for HeapEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min-heap via BinaryHeap
        match other.key[..other.key_len].cmp(&self.key[..self.key_len]) {
            std::cmp::Ordering::Equal => other.run_idx.cmp(&self.run_idx),
            o => o,
        }
    }
}

impl PartialOrd for HeapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn k_way_merge(run_paths: &[PathBuf], output: &Path, key_len: usize) -> Result<()> {
    let mut readers: Vec<BufReader<File>> = run_paths
        .iter()
        .map(|p| Ok(BufReader::with_capacity(1024 * 1024, File::open(p)?)))
        .collect::<Result<Vec<_>>>()?;

    let mut heap = BinaryHeap::new();
    for (i, reader) in readers.iter_mut().enumerate() {
        match read_one_record(reader, key_len) {
            Ok(rec) => heap.push(HeapEntry {
                key: rec.key,
                key_len: rec.key_len,
                blob: rec.blob,
                run_idx: i,
            }),
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(e) => return Err(e),
        }
    }

    let mut out = BufWriter::with_capacity(8 * 1024 * 1024, File::create(output)?);
    while let Some(entry) = heap.pop() {
        out.write_all(&entry.key[..entry.key_len])?;
        out.write_all(&(entry.blob.len() as u64).to_le_bytes())?;
        out.write_all(&entry.blob)?;
        let i = entry.run_idx;
        match read_one_record(&mut readers[i], key_len) {
            Ok(rec) => heap.push(HeapEntry {
                key: rec.key,
                key_len: rec.key_len,
                blob: rec.blob,
                run_idx: i,
            }),
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(e) => return Err(e),
        }
    }
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::EdgeType;
    use crate::write_columnar_from_nodes_edges;
    use tempfile::TempDir;

    #[test]
    fn spill_compile_digest_matches_vec_path() {
        let a = Node::new(NodeType::Function, "a");
        let b = Node::new(NodeType::Function, "b");
        let a_id = a.id;
        let b_id = b.id;
        let e1 = Edge::new(a_id, b_id, EdgeType::Calls);
        let e2 = Edge::new(b_id, a_id, EdgeType::Calls);

        let tmp = TempDir::new().unwrap();
        let mut spill = SegmentedSpill::create(tmp.path().join("spill")).unwrap();
        // Append out of order to exercise sort.
        spill.append_node(&b).unwrap();
        spill.append_node(&a).unwrap();
        spill.append_edge(&e2).unwrap();
        spill.append_edge(&e1).unwrap();
        let finished = spill.finish().unwrap();

        let path_spill = tmp.path().join("from_spill.bin");
        let path_vecs = tmp.path().join("from_vecs.bin");
        let d_spill = write_columnar_from_spill(finished, &path_spill).unwrap();
        let d_vecs = write_columnar_from_nodes_edges(vec![a, b], vec![e1, e2], &path_vecs).unwrap();
        assert_eq!(d_spill, d_vecs);
    }
}
