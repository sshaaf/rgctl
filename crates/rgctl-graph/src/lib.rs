//! Graph storage and query layer for rgctl.
#![warn(missing_docs)]

/// Graph storage backends and the [`backend::GraphBackend`] trait.
pub mod backend;
/// High-level [`code_graph::CodeGraph`] API.
pub mod code_graph;
/// Code location hashing and lookup helpers.
pub mod code_index;
/// Columnar mmap snapshot format (v2).
pub mod columnar_snapshot;
/// Hash-keyed blob vault for truncated document bodies.
pub mod content_store;
/// Typed bidirectional CSR topology.
pub mod csr;
/// JSON import/export for graph snapshots.
pub mod export;
/// Streaming compaction of columnar snapshots with a delta segment.
pub mod graph_compactor;
/// String interning for index keys.
pub mod intern;
/// Lazy heap-backed collections for graph schema types.
pub mod lazy_collections;
/// Snapshot version migration helpers.
pub mod migration;
/// On-disk `.rgctl/` artifact paths and `RGCTL_*` env helpers.
pub mod paths;
/// Mini query language over [`backend::MemoryBackend`].
pub mod query;
/// Node, edge, and graph schema types.
pub mod schema;
/// Append-only extract spill + external-sort compile to columnar.
pub mod segmented_spill;
/// Prepared and memory-mapped snapshot I/O.
pub mod snapshot;
pub mod structural_sketch;

pub use code_graph::CodeGraph;
pub use code_index::{CodeIndex, CodeLocation, hash_code};
pub use columnar_snapshot::{
    COLUMNAR_SNAPSHOT_VERSION, ColumnarGraphMmap, write_columnar_from_backend,
    write_columnar_from_nodes_edges,
};
pub use content_store::{
    CONTENT_STORE_FILE, ContentStore, INLINE_BODY_MAX_BYTES, hash_bytes, hash_text,
};
pub use csr::{CodeGraphCsr, edge_type_from_u8, edge_type_to_u8};
pub use export::{GraphSnapshot, export_json, export_json_to, import_json};
pub use query::{execute, execute_chunks, stream_query, QueryStream};
pub use graph_compactor::{
    CompactStats, DeltaSegment, GraphCompactor, compact_repo_snapshot, compact_snapshot_file,
};
pub use migration::{migrate_snapshot, migrate_v1_to_v2};
pub use schema::{AccessType, CallType, GRAPH_SCHEMA_VERSION, GraphParameter, SharedStr};
pub use segmented_spill::{
    DEFAULT_SORT_RUN_BYTES, FinishedSpill, SegmentedSpill, write_columnar_from_spill,
};
pub use snapshot::{
    MmappedGraphSnapshot, PreparedGraphSnapshot, PreparedIndexes, SNAPSHOT_FILE, SnapshotNodeStore,
};
pub use structural_sketch::{
    MIN_TOKEN_LEN, TOKEN_BLOOM_BITS, TOKEN_BLOOM_WORDS, TokenBloom, build_token_bloom, empty_bloom,
    keyword_in_bloom, keyword_overlap_score, satisfies_keyword_and, tokenize_string_into,
};

/// Normalize path separators for consistent comparison.
pub fn normalize_path_str(path: &str) -> String {
    path.replace('\\', "/")
}
