# Comprehensive Optimization Plan: `rgctl-analysis`

## Evaluation of the Specification Against Actual Code & Regression Findings

This plan cross-references the proposed optimization specification against:
- The **58 unstaged modified files** already in the working tree
- The **three root-cause regressions** found in the previous review
- The **actual current architecture** (CSR topology, dense flat arrays, mmap snapshots)

Each specification proposal is assessed as: ACCEPT, ACCEPT-WITH-MODIFICATIONS, ALREADY-DONE,
REJECT, or DEFER.

---

## Part A: Regression Fixes (Must Land First)

These three issues are actively causing wall-time regression on large codebases and must be
resolved before any new optimization work proceeds.

### R1. `FlatGraphIndex.out_adj` eagerly built but unused by PageRank

**Status:** Regression introduced in current unstaged changes.

**File:** `centrality.rs:112-120`

`FlatGraphIndex::from_view` now unconditionally builds `out_adj` (150K `Vec<usize>` +
500K entries on a large graph). `FastPageRank::compute_flat` never reads `out_adj` -- it
uses `flat_edges` directly. The allocation pollutes L3 cache before PageRank's hot loop.

**Fix:** Remove `out_adj` from `FlatGraphIndex`. Add a `build_out_adj(&self) -> Vec<Vec<usize>>`
method. Build it once in `compute_flat_centrality` and pass as a parameter to betweenness
and harmonic functions.

**Priority:** CRITICAL -- blocks all other centrality work.

---

### R2. `hyperball_exact` drops self-ball contents each round

**Status:** Correctness bug introduced in current unstaged changes.

**File:** `centrality_approx.rs:320`

The new double-buffer code does `next[node].clear()` then extends only from neighbors,
omitting `current[node]` (the node's own accumulated reachability from prior rounds). This
causes under-counted harmonic centrality on deep call chains.

**Fix:** Add `next[node].extend(current[node].iter().copied())` after `clear()`.

**Priority:** CRITICAL -- wrong results may trigger recomputation.

---

### R3. Bidirectional neighbor lists rebuilt inside diffusion loop

**Status:** Regression introduced in current unstaged changes.

**File:** `semantic_diffuse.rs:67-83`

The `bidirectional_neighbors` `Vec<Vec<u32>>` is constructed inside `for _ in 0..config.iterations`,
allocating and sorting N vectors on every iteration. The topology is immutable.

**Fix:** Hoist the construction before the loop.

**Priority:** HIGH -- O(N * iterations) redundant sort+dedup on every discover.

---

### R4. `macro_call_lookup.rs` writes empty `"[]"` strings instead of serialized data

**Status:** Data loss bug introduced in current unstaged changes.

**File:** `macro_call_lookup.rs:627-631`

Four JSON columns are hardcoded to `"[]"` instead of serialized blast-radius data. This
silently drops cached blast results, which may cause downstream re-computation.

**Fix:** Revert the 8 affected lines to use `serde_json::to_string(...)`.

**Priority:** HIGH -- silent data loss.

---

## Part B: Evaluation of Specification Proposals

### Subsystem 1: AST Parsing & Local Facts Extraction

#### 1a. Jump directly to function node via `named_descendant_for_byte_range`

**Verdict: ACCEPT-WITH-MODIFICATIONS**

The proposal to avoid O(M*N) tree walks is sound. However, the current code already has
`ParsedSourceFile` in `cfg_builder.rs` that indexes function locations to byte spans and
reuses the tree. The optimization is to use `named_descendant_for_byte_range` instead of
the recursive `find_function_by_name` walk, which is a targeted change.

**Caveat:** `named_descendant_for_byte_range` may not return the exact function node
(could return a parent or child). Requires validation against the `function_kinds_for`
table for each language.

#### 1b. Thread-local parser pooling

**Verdict: ACCEPT**

Currently `parse_source` creates a new parser per call. Tree-sitter parsers are
heavyweight (allocate internal memory buffers). `thread_local!` storage with per-Rayon-worker
parser reuse is a clean win. No design conflicts.

#### 1c. Group field-write records by file path, parse once per file

**Verdict: ACCEPT**

The diff shows `field_write.rs` has only test changes (String literal cleanup). The
production code still parses per-function. Grouping by file path and parsing once per
Rayon thread is correct and non-invasive.

---

### Subsystem 2: Intra-Procedural Dataflow & Slicing

#### 2a. Word-level BitSet arithmetic in reaching definitions

**Verdict: ALREADY-DONE (partially)**

The current unstaged diff for `dataflow.rs` already converts gen/kill/in/out sets from
`HashSet<Definition>` to `BitSet`, and uses `union_with`. However, the proposed Blueprint B
suggests `difference_with(kill_b)` + `union_with(gen_b)` which is more efficient than the
current `for def_idx in in_b.iter() { if !kill_b.contains(def_idx) { out_b.insert(def_idx) } }`.

**Remaining work:** Replace the element-by-element kill check with:
```rust
let mut out_b = in_b.clone();
out_b.difference_with(kill_b);
out_b.union_with(gen_b);
```
This is a 3-line change that eliminates the per-element branch.

#### 2b. Index reaching definitions by variable name in PDG

**Verdict: ACCEPT**

The current `pdg.rs` diff adds variable interning (`intern_var` + `var_intern` HashMap)
for dedup edges, but PDG data-dependency construction still does linear scans over
reaching definitions. Adding a `HashMap<u32, Vec<usize>>` (interned var ID -> definition
indices) per block would make the inner loop O(1) per variable.

#### 2c. Reverse Post-Order for dominator convergence

**Verdict: DEFER**

The dominance module is not in the unstaged diffs and is not on the hot path for discover.
RPO ordering is a correctness improvement more than a performance one for typical software
CFGs. Low priority.

#### 2d. Integer union-find for may-alias

**Verdict: ALREADY-DONE**

The unstaged diff for `alias.rs` already implements this: `VarIntern` struct, integer
`Vec<u32>` parent array, `uf_find`/`uf_union` on `u32` indices. No further work needed.

---

### Subsystem 3: Centrality & Graph Algorithms

#### 3a. Parallel PageRank (Blueprint A)

**Verdict: ACCEPT-WITH-MODIFICATIONS**

The specification's Blueprint A proposes:
1. Pre-compute per-node `contrib = damping * (rank / out_degree)` in parallel
2. Rayon parallel reductions for `dangling_mass` and `max_delta`
3. Contiguous `Vec<f32>` arrays

Assessment against actual code:

- The current `FastPageRank::compute_flat` uses `flat_edges` iteration (sequential), which
  is **already cache-friendly** (linear scan over contiguous `(usize, usize)` pairs). The
  spec's proposal to iterate `out_adj` instead would be **worse** for cache because
  `Vec<Vec<usize>>` has pointer indirection per node.

- **Pre-computing `contrib`** is valid: hoisting the division `rank / out_degree` out of
  the edge loop eliminates one FP division per edge per iteration. This is the biggest
  single-threaded speedup available.

- **Rayon `par_iter` for the edge propagation loop** is **problematic**: `next_ranks[dst] +=`
  is a data race when multiple edges target the same node. The spec's Blueprint A applies
  contrib sequentially (step 4 is a sequential `for src`), which is correct but misses the
  point of parallelism. True parallel scatter requires atomics or per-thread partial arrays.

- **`f32` instead of `f64`**: Acceptable for PageRank convergence tolerance 1e-4 on large
  graphs. The columnar results table already stores `f32`. But `f64` should remain for the
  accumulation loop to avoid precision loss during summation; convert to `f32` only at the
  output boundary.

**Recommended approach:**
1. Pre-compute `node_contrib[src] = damping * rank[src] / out_degree[src]` (parallel).
2. Keep the edge propagation loop sequential (it's scatter-add, not parallelizable without
   atomics, and the flat_edges scan is already cache-friendly).
3. Compute `dangling_mass` and `max_delta` with Rayon parallel reductions.
4. Keep `f64` internally; produce `f32` only into `CentralityTable`.

#### 3b. Parallel sampled betweenness

**Verdict: ACCEPT**

`brandes_single_source` is embarrassingly parallel across pivot sources. Each pivot produces
an independent `Vec<f64>` partial score array. The current code runs pivots sequentially.

```rust
let betweenness: Vec<f64> = pivots
    .par_iter()
    .map(|&source| brandes_single_source(&out_adj, source, n))
    .reduce(
        || vec![0.0; n],
        |mut a, b| { a.iter_mut().zip(&b).for_each(|(x, y)| *x += y); a }
    );
```

No data races, no shared mutable state. Pure win.

**Caveat:** `brandes_single_source` allocates 6 `Vec` arrays per call. For true zero-alloc,
pre-allocate buffers per Rayon thread via `fold`. But even without this, the parallelism
gain outweighs the allocation cost for 512 pivots on 8+ cores.

#### 3c. Parallel HyperBall propagation

**Verdict: ALREADY-DONE (for HLL path)**

The current `hyperball_hll_parallel` already uses `next.par_iter_mut().enumerate().for_each()`
for the HyperLogLog merge step. The exact path (`hyperball_exact`) is sequential but only
runs for graphs <= 8192 nodes, where parallelism overhead would exceed the gain.

No further work needed.

#### 3d. Community detection with CSR and flat labels

**Verdict: ALREADY-DONE (partially)**

The current unstaged diff for `community.rs` already converts `HashMap<usize, f64>` to
`Vec<f64>` for `internal_by_community` and `degree_sum_by_community`. The label propagation
already uses `Vec<usize>` labels and dense flat neighbor lists via `StructuralTopology`.

**Remaining work:** The modularity calculation loop could be parallelized with Rayon, but
it's O(C) where C = community count (typically small). Low priority.

---

### Subsystem 4: Blast Radius & SCC Condensation

#### 4a. Dense Vec for node-to-SCC mapping

**Verdict: ACCEPT**

`blast_radius_scc.rs` line 1571 uses `HashMap<NodeIndex, usize>` for `node_to_scc_idx`.
Since `NodeIndex` values are dense `0..node_count`, this should be `Vec<usize>` with
direct array indexing. Eliminates hashing overhead in the DAG edge construction loop
(`view.for_each_edge`), which runs once per edge in the graph.

**Note:** The UUID-to-SCC mapping `node_to_scc: HashMap<Uuid, NodeIndex>` must remain a
HashMap because UUIDs are not contiguous. Only the `NodeIndex->SCC` direction can be
converted to Vec.

#### 4b. Sparse zstd-compressed reachability

**Verdict: ALREADY-DONE**

The current code already implements this: `ReachabilityRow` with zstd compression,
`ReachabilityStore` with lazy/on-demand modes, sparse row omission for self-only SCCs.
The unstaged diff improves `bitset_to_words` to iterate only set bits instead of scanning
all bit positions. No further work needed.

#### 4c. Mmap snapshot loading without full deserialization

**Verdict: ALREADY-DONE**

`BlastEngineSnapshot::read_graph_digest` already mmaps the file and parses only the
leading bincode string (graph digest) without deserializing reachability rows.
`ReachabilityStore::from_snapshot` with `Lazy` backing defers row decompression to query
time. No further work needed.

---

### Subsystem 5: Semantic Search & Matrix Diffusion

#### 5a. Parallel Hamming top-k search (Blueprint C)

**Verdict: ACCEPT**

The current `hamming_top_k` is purely sequential. Blueprint C's `par_chunks_exact` +
per-thread `BinaryHeap` + `reduce` merge is correct and applies directly. The Hamming
distance function already uses word-aligned `u64::count_ones()`.

One adjustment: the current `SemanticIndex` structure uses `binary_embeddings: Vec<u8>`.
The `par_chunks_exact(stride)` approach works directly on this layout.

#### 5b. Parallel Jacobi diffusion

**Verdict: ALREADY-DONE (partially)**

The current `semantic_diffuse.rs` already uses `next.par_chunks_mut(dims)` for the Jacobi
update step. The regression (R3) is that the neighbor list construction was accidentally
moved inside the iteration loop. After fixing R3, the diffusion itself is already parallel.

**Remaining work after R3 fix:** None for the core diffusion. The L2 normalization step
could also be parallelized but is O(N * dims) and already fast.

#### 5c. Async ONNX inference

**Verdict: DEFER**

The code-daemon ONNX inference path is not in the hot discover pipeline. It's used for
semantic search queries, which are latency-sensitive but not throughput-bound. Wrapping in
`spawn_blocking` is correct but low priority. The tokenizer reuse via `Arc` is already
the standard pattern in the codebase.

---

### Subsystem 6: Storage, Archiving & Serialization

#### 6a. Indexed CFG/PDG archive with mmap zero-copy reads

**Verdict: ACCEPT-WITH-MODIFICATIONS**

The current `cfg_pdg_archive.rs` mmaps the file but then fully deserializes via bincode
into heap-allocated `HashMap<Uuid, CfgPdgRecord>`. This is O(N_functions) at load time
with significant allocation.

The proposal to add a lightweight index file (`analysis_index.bin`) mapping function keys
to byte offsets is sound but requires a format change. A more incremental approach:

1. Add a TOC (table of contents) section to the existing archive format with
   `(Uuid, offset, length)` entries.
2. Keep the mmap alive and deserialize individual records on demand via
   `bincode::deserialize(&mmap[offset..offset+length])`.
3. Use `Arc<Mmap>` to share the mapping across threads.

This avoids a second file and maintains backward compatibility by adding the TOC as a
new section after the existing payload.

#### 6b. Columnar results output with parallel bincode save

**Verdict: ALREADY-DONE (partially)**

The columnar results system (`AnalysisResults`, `CentralityTable`, etc.) already writes
flat arrays directly. The `analyze_columnar` path fills columns in-place without UUID
HashMaps. The remaining gap is that the final disk write could be async (Tokio), but
this is a small fraction of total discover time.

---

### Specification Architecture: Tokio + Rayon Runtime

**Verdict: ACCEPT the principle, REJECT the scope**

The architecture of "Tokio for I/O, Rayon for compute, `spawn_blocking` as bridge" is
correct and already partially in use (the daemon/serve path uses Tokio; discover uses
Rayon). However:

- **Do not convert the CLI discover pipeline to Tokio.** It is a batch process where async
  I/O adds complexity without benefit. File reads are dominated by mmap (zero-copy, no
  async needed). Rayon parallelism alone handles the compute.
- **The daemon/serve path** should use `spawn_blocking` for analysis queries. This is
  already the case for GQL query execution.
- **Do not add Tokio as a dependency to `rgctl-analysis`.** Keep it in the CLI/daemon
  crate only.

---

## Part C: Phased Implementation Plan

### Phase 0: Fix Regressions (Prerequisite)

| Task | File | Effort | Risk |
|------|------|--------|------|
| R1: Remove `out_adj` from `FlatGraphIndex`, add `build_out_adj()` | centrality.rs | Low | Low |
| R2: Add `next[node].extend(current[node]...)` in hyperball_exact | centrality_approx.rs | Trivial | Low |
| R3: Hoist bidirectional neighbor construction out of loop | semantic_diffuse.rs | Trivial | Low |
| R4: Revert macro_call_lookup `"[]"` to serialized JSON | macro_call_lookup.rs | Trivial | Low |

**Verification:** Run `cargo test --release -p rgctl-analysis`. Run discover on a
medium codebase and verify wall time returns to baseline.

---

### Phase 1: Sequential Hot-Path Optimizations (No New Dependencies)

These are pure algorithmic improvements within existing single-threaded code.

| # | Task | File | Spec Ref | Effort |
|---|------|------|----------|--------|
| 1.1 | Pre-compute PageRank `node_contrib` (hoist FP division) | centrality.rs | 3a | Low |
| 1.2 | Use `difference_with`/`union_with` in reaching defs | dataflow.rs | 2a | Trivial |
| 1.3 | Dense Vec for NodeIndex->SCC in blast_radius_scc | blast_radius_scc.rs | 4a | Low |
| 1.4 | Index reaching defs by interned variable ID in PDG | pdg.rs | 2b | Medium |
| 1.5 | Thread-local tree-sitter parser pooling | cfg_builder.rs | 1b | Low |
| 1.6 | Group field-write records by file, parse once | field_write.rs | 1c | Medium |

**Verification:** Benchmark before/after with `benches/analysis_benchmarks.rs`. No new
test failures.

---

### Phase 2: Rayon Parallelism (CPU-Bound Passes)

These add `rayon` parallel iterators to compute-heavy passes. Rayon is already a dependency.

| # | Task | File | Spec Ref | Effort |
|---|------|------|----------|--------|
| 2.1 | Parallel sampled betweenness (pivots in par_iter) | centrality_approx.rs | 3b | Low |
| 2.2 | Parallel Hamming top-k search (par_chunks + heap merge) | semantic_search.rs | 5a | Medium |
| 2.3 | Parallel PageRank dangling_mass + max_delta reductions | centrality.rs | 3a | Low |
| 2.4 | Parallel brandes_single_source buffer reuse (fold) | centrality_approx.rs | 3b | Medium |
| 2.5 | File-level par_iter for AST parsing in field_write | field_write.rs | 1c | Medium |

**Verification:** Run `cargo test --release`. Verify scaling with `RAYON_NUM_THREADS=1`
vs `RAYON_NUM_THREADS=8`. Expect >= 3x speedup on betweenness and Hamming search.

---

### Phase 3: Storage & Archive Efficiency

| # | Task | File | Spec Ref | Effort |
|---|------|------|----------|--------|
| 3.1 | Add TOC section to cfg_pdg_archive for on-demand record deser | cfg_pdg_archive.rs | 6a | High |
| 3.2 | Use `named_descendant_for_byte_range` for function lookup | cfg_builder.rs | 1a | Medium |

**Verification:** Compare archive load time before/after. Verify CFG construction produces
identical results for all supported languages.

---

### Phase 4: Deferred / Future Work

| # | Task | Spec Ref | Reason for Deferral |
|---|------|----------|---------------------|
| 4.1 | Async ONNX inference wrapper | 5c | Not on discover hot path |
| 4.2 | RPO ordering for dominator convergence | 2c | Low impact on real CFGs |
| 4.3 | Tokio runtime integration in CLI | Architecture | Batch CLI doesn't benefit from async |
| 4.4 | `f32` PageRank accumulation | 3a | Precision risk; `f64` internal is safer |
| 4.5 | Parallel modularity calculation | 3d | O(C) where C is small |

---

## Part D: What the Specification Gets Wrong

### D1. Blueprint A step 4 claims to parallelize edge propagation but doesn't

The Blueprint A code shows step 4 as a sequential `for src in 0..node_count` loop with
`next_ranks[dst] += contrib`. This is correct for avoiding data races, but the spec text
claims "Contiguous O(1) accesses" as if parallelism is happening. The actual sequential
flat_edges scan in the current code is already O(1) per access and more cache-friendly
than the `out_adj[src]` indirection in the Blueprint.

**Resolution:** Keep the current `flat_edges` iteration for the scatter step. Only
parallelize the pre-compute, dangling-mass, and convergence-check steps.

### D2. The spec proposes `Vec<f32>` for PageRank accumulation

Using `f32` for the internal accumulation loop risks precision loss during summation of
many small rank contributions. The Kahan summation or mixed-precision approach (accumulate
in `f64`, store results in `f32`) is safer. The columnar output table already stores `f32`,
so the conversion cost is negligible.

### D3. The spec proposes "No Locks, Zero Alloc" but the dataflow BitSet approach still allocates

Blueprint B shows `BitSet::new()` calls inside the worklist loop. `BitSet` internally
allocates a `Vec<u32>`. True zero-alloc would require pre-sized bitset pools. In practice,
the BitSet allocation is amortized and not the bottleneck.

### D4. The spec's async architecture diagram implies Tokio wraps the entire discover pipeline

The discover pipeline is a batch process: read files -> parse -> analyze -> write results.
Adding Tokio's event loop here adds task-scheduling overhead and complexity without benefit.
Rayon alone handles the parallelism. Tokio should remain confined to the daemon/serve path.

### D5. Memory allocation verification via valgrind is impractical for large repos

The spec suggests running `discover` on the Linux kernel under valgrind. Valgrind's 10-50x
slowdown makes this infeasible for multi-million-node graphs. Use `perf stat` or
`jemalloc`'s `malloc_stats` instead for allocation profiling.

---

## Part E: Summary Scoring

| Spec Proposal | Verdict | Already Done? | Priority |
|---------------|---------|---------------|----------|
| 1a. Byte-range function lookup | ACCEPT-MOD | No | Phase 3 |
| 1b. Thread-local parser pool | ACCEPT | No | Phase 1 |
| 1c. File-grouped field-write | ACCEPT | No | Phase 1-2 |
| 2a. BitSet word-level dataflow | ACCEPT | Partially | Phase 1 |
| 2b. Variable-indexed reaching defs | ACCEPT | No | Phase 1 |
| 2c. RPO dominator convergence | DEFER | No | Phase 4 |
| 2d. Integer union-find alias | ALREADY-DONE | Yes | -- |
| 3a. Parallel PageRank | ACCEPT-MOD | No | Phase 1-2 |
| 3b. Parallel betweenness | ACCEPT | No | Phase 2 |
| 3c. Parallel HyperBall | ALREADY-DONE | Yes | -- |
| 3d. CSR community detection | ALREADY-DONE | Partially | -- |
| 4a. Dense Vec for SCC mapping | ACCEPT | No | Phase 1 |
| 4b. Sparse zstd reachability | ALREADY-DONE | Yes | -- |
| 4c. Mmap blast snapshot | ALREADY-DONE | Yes | -- |
| 5a. Parallel Hamming search | ACCEPT | No | Phase 2 |
| 5b. Parallel Jacobi diffusion | ALREADY-DONE | Yes (after R3) | -- |
| 5c. Async ONNX inference | DEFER | No | Phase 4 |
| 6a. Indexed CFG/PDG archive | ACCEPT-MOD | No | Phase 3 |
| 6b. Columnar results output | ALREADY-DONE | Partially | -- |
| Tokio runtime architecture | REJECT for CLI | Partially (daemon) | -- |
