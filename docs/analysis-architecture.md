# rgctl-analysis Architecture

The `rgctl-analysis` crate bridges the code knowledge graph (`rgctl-graph`) with static-analysis algorithms. It organizes graph usage into three tiers.

## Tier 1: Repository graph (source of truth)

- **`MemoryBackend`** — in-memory node/edge store with typed topology
- **Mmap snapshots** — `PreparedGraphSnapshot`, `MmappedGraphSnapshot` for zero-copy reads

Nodes represent symbols (functions, types, modules). Edges carry `EdgeType` semantics (`Calls`, `Uses`, `Contains`, `References`, etc.).

## Tier 2: Projected views (repository-level analysis)

| Type | Module | Purpose |
|------|--------|---------|
| `PetGraphView` | `graph_utils` | Façade over typed bidirectional CSR (`StructuralTopology`); UUID maps |
| `StructuralTopology` / `CodeGraphCsr` | `structural_topology` / `rgctl-graph::csr` | Hot edge residency (~5 B/edge/dir); community/centrality/blast/dependency |
| `ColdMetadataDb` | `cold_metadata` | Mmap-backed node payloads after early snapshot write |
| `CallGraph` | `callgraph` | u32-indexed call-only adjacency for fast traversal |
| `FlatGraphIndex` | `centrality` | Contiguous `usize` edge list for numeric algorithms |

**Convention:** Discover ingest uses **segmented disk spill**
(`GraphBuilder::with_spill` → `write_columnar_from_spill`): nodes/edges are
append-only length-prefixed bincode on disk, externally sorted, then compiled to
columnar v2 — no full `Vec<Node>`/`Vec<Edge>` or `MemoryBackend` during discover.
Analysis opens `ColdMetadataDb` + CSR from the mmap; hydrate a `CodeGraph` only for
`--with-dashboard` / migration / JSON. Share `&PetGraphView` / `StructuralTopology`
across community / centrality / blast, then drop the view after the SCC engine is built.

**Incremental updates:** when `.rgctl/graph.snapshot.bin` exists,
[`IncrementalUpdater`](../crates/rgctl-incremental/src/updater.rs) extracts only
changed files into a [`DeltaSegment`](../crates/rgctl-graph/src/graph_compactor.rs),
stream-compacts with [`GraphCompactor`](../crates/rgctl-graph/src/graph_compactor.rs)
(alive UUID filter + edge row streaming; name/type indexes rebuilt during spill compile),
atomically renames the new snapshot, then reloads. Force rebuild uses
`process_repository_to_snapshot`. Without a snapshot, the legacy in-memory edit path remains.
`write_columnar_from_backend` / `process_repository` remain for callers that need a live backend.

### Columnar content digest and cache invalidation

Columnar v2 headers store a 64-byte hex BLAKE3 **content digest** (not a hash of file
layout/offsets). Writers (discover spill compile, `write_columnar_from_*`, compact)
compute it as:

1. `bincode(Node)` for every live node, **sorted by UUID**
2. then `bincode(Edge::for_columnar_digest())` for every live edge, **sorted by
   `(from, to, edge_type)`**

Edges are **topology-only** for the digest: columnar rows do not store edge properties
(`call_site_line`, etc.), so hashing strips them. Compact **recomputes** this digest
over the final live set (same helpers as spill compile) — never
`hash(old_digest || delta)`.

Sidecars under `.rgctl/` that embed `graph_digest` (`blast_engine.snapshot.bin`,
macro call index/lookup, `analysis_results.bin`, semantic index, CFG/PDG archive) are
**deleted after compact** so the next discover / blast-radius / macro path rebuilds
against the new header instead of serving a stale hit. Do not copy the old digest into
a new snapshot header to “preserve” those caches.

### Repository-level analyses

| Analysis | Primary graph | Algorithm |
|----------|---------------|-----------|
| Centrality | `PetGraphView` → `FlatGraphIndex` | PageRank, sampled Brandes betweenness, HyperBall harmonic |
| Community | `PetGraphView` (undirected filter) | Label propagation + modularity ([naming note](design/graph-metrics-design.md#31-community-detection-naming)) |
| Blast radius (engine) | CSR Calls filter + kosaraju | Kosaraju SCC → on-demand reachability (flat graphs) or bitset rows |
| Blast radius (analyzer) | `PetGraphView` Calls filter | Reverse BFS |
| Dependencies | `PetGraphView` directed | Kosaraju SCC, reverse BFS impact |
| Complexity | Backend node properties | Aggregation |
| Migration | Community graph | Weighted topological sort |

## Tier 3: Intra-procedural graphs

| Type | Module | Purpose |
|------|--------|---------|
| `ControlFlowGraph` | `cfg`, `cfg_builder` | Per-function CFG via tree-sitter |
| `ProgramDependenceGraph` | `pdg` | Data + control dependencies |
| `DominatorTree` | `dominance` | Immediate dominators and frontiers |

### Pipeline

```
tree-sitter AST → CFG → DominatorTree + ReachingDefs → PDG → Slicing / Taint
```

`InterproceduralCFG` stitches per-function CFGs with `CallGraph` for cross-function slicing.

## Traversal depth

Graph BFS traversals (blast radius analyzer, dependency impact) share `TraversalConfig` with default depth **10** (`DEFAULT_TRAVERSAL_DEPTH` in `graph_utils`). Use `TraversalConfig::unlimited()` for full transitive closure; prefer `BlastRadiusEngine` for large graphs.

## Caching and persistence

- **`FlowCache`** / **`CfgPdgArchive`** — per-function CFG/PDG cache
- **`BlastEngineSnapshot`** — persisted SCC reachability bitsets (keyed by `graph_digest`)
- **`AnalysisResults`** — columnar metrics decoupled from graph topology (`CentralityTable`, community, blast tables)
- **`MacroCallIndex` / lookup DB** — digest-gated; invalidated after snapshot compact

See [Columnar content digest and cache invalidation](#columnar-content-digest-and-cache-invalidation) above.

### Centrality pipeline (discover)

Discover uses **`CentralityAnalyzer::analyze_columnar`**: flat scores from `FlatGraphIndex` are written directly into `AnalysisResults` without intermediate `HashMap<Uuid, _>` handoffs.

| Graph size | PageRank | Betweenness | Harmonic |
|------------|----------|-------------|----------|
| V ≤ 500 | Exact (20 iter, ε=1e-6) | Exact Brandes | Exact BFS |
| 500 < V ≤ 500,000 | Exact (20 iter) | Sampled Brandes (k=512) | HyperBall (h=16, parallel HLL) |
| V > 500,000 | Gated (8 iter, ε=1e-4) | Sampled Brandes (k=512) | HyperBall (h=8, parallel HLL) |

Constants: `LARGE_GRAPH_PAGERANK_*`, `LARGE_GRAPH_HYPERBALL_*` in `centrality.rs` / `centrality_approx.rs`.

**Profiling:** `discover -v` with `RUST_LOG=profile=info` emits `[profile] stage` and `[profile] centrality sub-phase` lines (PageRank, betweenness, harmonic, columnar fill timings). Harmonic runs only with `discover --with-harmonic`.

See [internal/profile.md](internal/profile.md) for cold-profile commands and developer-machine timings; [harmonic-centrality.md](harmonic-centrality.md) for HyperBall detail.

## Further reading

- Crate README: `crates/rgctl-analysis/README.md`
- OpenSpec design: `openspec/changes/review-rgctl-analysis/design.md`
- Benchmarks: `cargo bench --bench graph_benchmarks` (workspace root)
