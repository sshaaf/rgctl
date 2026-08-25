# Approximate Centrality Algorithms (rgBuilder)

Design note for sampled betweenness and HyperBall harmonic centrality in
`crates/rgbuilder-analysis/src/centrality_approx.rs`, and the columnar discover
path in `centrality.rs` / `results.rs`.

## Motivation

Exact betweenness (Brandes) and exact harmonic (all-pairs BFS) are **O(V × (V + E))**.
On graphs above ~500 nodes this becomes prohibitive; on 500k nodes it is days of CPU.

Production static-analysis tools use **sampled** and **sketch-based** estimators that
preserve **ranking quality** for architectural hotspots while running in seconds.

rgBuilder uses a **tiered strategy**:

| Graph size | Betweenness | Harmonic |
|------------|-------------|----------|
| V ≤ 500 | Exact Brandes (all sources) | Exact BFS from all sources |
| V > 500 | Sampled Brandes (RANDES) | HyperBall + HyperLogLog |
| V ≤ 8,192 (harmonic only) | — | Exact set propagation inside HyperBall |

Defaults: `k = 512` pivots, `h = 16` HyperBall rounds (capped to **8** when V > 500,000),
HLL precision `p = 14` (adaptive below).

---

## Discover columnar path

`discover` calls **`CentralityAnalyzer::analyze_columnar`**, which:

1. Builds one `FlatGraphIndex` and runs PageRank / betweenness / harmonic on flat `Vec`s.
2. Writes scores via **`AnalysisResults::fill_centrality_from_flat`** (compact-ID indexed arrays).
3. Emits **`CentralityApproxStats::log_profile`** when `RUST_LOG=profile=info`.

This avoids multi-million-entry `HashMap<Uuid, CentralityScores>` allocations that previously
spiked peak RSS on kernel-scale graphs.

`rgctl metrics` still uses **`analyze_with_view`** (HashMap report) but shares the same flat
compute core and adaptive gating.

### rgbuilder-analysis-perf (2026-08-13)

Phase 1–3 implementation complete (`openspec/changes/rgbuilder-analysis-perf/`):

- **Shared `out_adj`** on `FlatGraphIndex`; exact Brandes/Harmonic reuse flat buffers; HyperBall exact double-buffer; Fisher–Yates pivots when `k ≥ n/2`.
- **Blast BFS** uses pre-built function-id set + `with_node` for name/complexity paths; reachability LRU promote O(1); `bitset_to_words` via set iterator.
- **Alias** integer union-find; **community** modularity flat `Vec<f64>`; **semantic diffuse** borrows callee slices.
- **Dataflow** BitSet IN/OUT over global def index; **PDG** interned `u32` dedup keys; **CFG** monotonic block IDs + exception-edge `HashSet`.
- **Macro lookup** BLOB-only writes; **handoff** `resolve_handoff_seeds_with_call_graph`.

Cold profiles: see `openspec/changes/rgbuilder-analysis-perf/PROFILE.md`. Linux centrality ~2.65 s wall; betweenness ~2.13 s on 2.66M nodes.

---

## Adaptive gating (V > 500,000)

| Metric | Default (V ≤ 500k) | Large graph (V > 500k) |
|--------|--------------------|-------------------------|
| PageRank iterations | 20 | **8** |
| PageRank tolerance ε | 1e-6 | **1e-4** |
| HyperBall rounds | 16 (or configured) | **8** |

Policy and migration use **relative rank order** and community aggregates — not bit-identical
PageRank convergence on multi-million-node call graphs. Explicit CLI tuning remains available:

```bash
rgctl -f json metrics --pagerank --iterations 50
```

---

## Technique A: Sampled Betweenness (RANDES / Eppstein–Wang)

### Algorithm

1. Build a **flat directed adjacency list** from the behavioral edge projection
   (`FlatGraphIndex` — same layout as PageRank).
2. Choose **k** pivot sources uniformly at random (seeded for reproducibility, default seed `0xA5A55A5AC3C33C3C`).
3. For each pivot `s`, run **one Brandes single-source pass**.
4. Sum partial scores across pivots and **scale** to estimate full betweenness.

### Complexity

- Sampled: **O(k × (V + E))** with k ≪ V (default k = 512)

### Implementation

- `SampledBetweenness::compute_flat(index, k, seed)`
- Wired when `V > exact_limit` (500)

---

## Technique B: HyperBall Harmonic Centrality

### Definition

Normalized **out-harmonic centrality** on directed graph G:

```
H(u) = (1 / (|V| - 1)) × Σ_{v ≠ u, d(u,v) < ∞} 1 / d(u,v)
```

### HyperBall idea

Propagate **reachability sketches** for `h` rounds. Each node maintains a **HyperLogLog (HLL)**
sketch; merge approximates set union cardinality. Early-stop when no ball grows.

### Two internal paths

| V | Method | Why |
|---|--------|-----|
| ≤ 8,192 | `hyperball_exact` — `HashSet` propagation | HLL biased on tiny graphs |
| > 8,192 | `hyperball_hll_parallel` — parallel HLL merge | Rayon scatter over nodes per round |

### Parallel implementation

For V > 8,192, each propagation round uses **Rayon** over nodes:

```text
next[node] = HLL({node}) ∪ merge(current[neighbor] for neighbor in out_adj[node])
```

Reads from `current` are shared; each thread writes only its `next[node]`. The convergence
scan (estimate + harmonic accumulation) remains sequential O(V).

### HyperLogLog sketch

- Adaptive precision: p=14 (V ≤ 8k), p=12 (V ≤ 100k), p=10 (V > 100k)
- Double-buffered `current` / `next` with in-place `reset`

### Complexity

- HyperBall HLL (parallel): **O(h × E × m / cores)** register merges per round (memory-bandwidth bound)

---

## Integration summary

```
discover → analyze_columnar → FlatGraphIndex
                              → FastPageRank (flat Vec)
                              → SampledBetweenness / HyperBallHarmonic (flat Vec)
                              → fill_centrality_from_flat → analysis_results.bin
```

### Dashboard / `function_metrics.json`

- Scores live in **`analysis_results.bin`** (columnar `CentralityTable`).
- Graphs with **≥ 50,000** source nodes export **`function_metrics.json`** in
  `sparse_mode: "community_only"` (metagraph + WASM carry per-function metrics).
- **`DashboardExportContext`** passes in-memory `AnalysisResults` during discover to avoid
  reloading analysis from disk for each export stage.

### Configuration (future)

Planned `rgctl.toml` keys:

```toml
[centrality]
exact_limit = 500
sample_pivots = 512
hyperball_rounds = 16
sample_seed = 0xA5A55A5AC3C33C3C
```

Currently hard-coded via `DEFAULT_*` and `LARGE_GRAPH_*` constants.

---

## Scale measurements (release build, Jul 2026)

### Linux kernel (`example/linux`, ~2.65M nodes default discover, 8.56M edges)

> **Note (Aug 2026 / perf-ingest-scale-gates):** Layer F field `Variable` nodes are **gated behind `--with-cfg`**. Default cold discover no longer materializes C struct fields as graph nodes (restores pre–Layer F node counts). CSR builds from mmap via streaming `for_each_edge` (no full edge `Vec`).
>
> **Note (Jul 2026 / #29 + #31):** default `discover` skips HyperBall harmonic and dashboard export.
> Expected cold wall ≈ **~130–155s** without field materialization.
> Use `--with-harmonic` / `--with-dashboard` / `--with-cfg` to restore those stages.

#### Cold profile gates

Manual tests: `cargo test --release --test cold_profile_gates -- --ignored --nocapture`

| Repo | Command | Gate |
|------|---------|------|
| `example/linux` | `discover . -v` | wall ≤ 170s, nodes ≤ 2.8M |
| `example/metasfresh-4.9.8b` | `discover . --with-cfg --with-security --with-taint -v` | wall ≤ 584s |
| `example/kafka` | `discover . -v` | wall ≤ baseline × 1.10 (`RGBUILDER_KAFKA_COLD_BASELINE_SECS`) |

#### Cold profile after CSR + backend drop (2026-07-20)

`RUST_LOG=info,profile=info` cold discover (no harmonic / no dashboard), log:
`example/linux/discover-profile-cold-csr.log`.

| Metric | Result |
|--------|--------|
| **Wall** | **147.8 s** (index 95s, post-index 34s) |
| **Peak RSS** | **15.8 GB** (periodic sampler high-water) |
| Topology (CSR build) | 1.13 s |
| Complexity | 1.30 s |
| Community | 4.89 s |
| Centrality (no harmonic) | 2.26 s (betweenness 1.9s) |
| Dependency | 4.71 s |
| Blast build | 3.46 s |
| Macro index | 0.008 s (bulk blast skipped on flat graph) |

**Reading the peak:** Lever 1 removed `MemoryBackend` co-residency; Lever 1.5
(segmented spill) removes full `Vec<Node>`/`Vec<Edge>` staging during discover ingest.
Extract appends length-prefixed bincode to `.rgbuilder/spill/`, then externally sorts and
compiles columnar. Absolute peak should move toward **resolution-map RAM + sort/compile
buffers** (not linear full-graph struct heap). Remaining multi-GB on Linux is largely
`symbol_index` / suffix maps until those are slimmed or spilled.

Discover `-v` reports **`ingest_peak_rss_mb`** vs **`analysis_peak_rss_mb`** separately
(`[profile] discover summary`).

Artifacts after this run: `.rgbuilder` ≈ 2.0 GB (`graph.snapshot.bin` 1.2G, `analysis_results.bin` 354M, `blast_engine.snapshot.bin` 259M).

Sub-phase profile (`RUST_LOG=profile=info discover -v`):

| Sub-phase | Before optimizations | After (parallel HyperBall + gating) | Default (no `--with-harmonic`) |
|-----------|---------------------|-------------------------------------|--------------------------------|
| PageRank | ~85s (with HashMap path) | **0.18 s** | same |
| Betweenness (sampled) | — | **2.0 s** | same |
| Harmonic (HyperBall) | **84.3 s** (16 rounds, sequential) | **31.0 s** (8 rounds, Rayon) | **skipped** |
| **Centrality total** | **~87 s** | **~33 s** | **~3 s** (PR + betweenness) |
| **Discover wall (incremental)** | **~140 s** | **~84 s** | lower by ~harmonic |
| **Discover wall (cold)** | **~354 s** | **~231 s** → re-profile **~169–172 s**; expected **~140 s** without harmonic | target after #29 |
| Peak RSS | 13.3 GB | **~14–17 GB** with HyperBall (old “5.5 GB after columnar” claim was about avoiding UUID HashMap centrality, **not** eliminating dual full-graph residency) | mid‑teens without harmonic until #33 materialization fixes |

### RSS materialization (#33) — Jul 2026

High RSS is **duplicate graph residency** (backend + prepared clone + petgraph view), not too many analysis passes.

| Fix | Status |
|-----|--------|
| Periodic peak RSS sampler (`MemoryMonitor::start_periodic_sampling`) | done |
| Write columnar snapshot from backend **without** `PreparedGraphSnapshot` clone | done (`write_columnar_from_backend`) |
| Drop undirected `UnGraph` + UUID HashMap→dense `Vec`; EdgeFiltered SCC (no call-only DiGraph clone) | done |
| Early mmap write → drop prepared; drop topology view after blast engine build | done |
| Full CSR topology replacing typed `DiGraph` for community/centrality/blast | **done** (`CodeGraphCsr` + `StructuralTopology`; `PetGraphView` is CSR façade) |
| CSR from mmap without `edge_topology_typed()` `Vec` | **done** (`CodeGraphCsr::from_store_topology`) |
| Layer F field `Variable` nodes gated on `--with-cfg` | **done** (default discover skips field materialization) |
| Cold profile gates (linux / metasfresh / kafka) | **done** (`tests/cold_profile_gates.rs`) |
| ColdMetadataDb (mmap) opened after early snapshot; **drop `CodeGraph` before community/centrality/blast** | **done** (hydrate only for `--with-dashboard` / migration / JSON) |
| **Lever 1: discover ingest skips `MemoryBackend`** — `write_columnar_from_nodes_edges` | **done** |
| **Lever 1.5: segmented disk spill** — `SegmentedSpill` + external sort + `write_columnar_from_spill`; `GraphBuilder::with_spill` | **done** (resolution HashMaps still in RAM) |
| **Delta-merge / GraphCompactor** — stream-filter base mmap + delta extract; indexes rebuilt in Pass 1/compile; atomic rename | **done** (`graph_compactor`; wired into `IncrementalUpdater`) |
| HyperBall harmonic | **opt-in** via `--with-harmonic` (keep) |

Do **not** invest in merging community+centrality+blast into one algorithm pass for RSS — invest in representation / lifetimes.

Top PageRank hotspot remained **BIT** (stable rank order).

### Smaller repos

| Repo | Nodes | Edges | Total centrality | Betweenness | Harmonic |
|------|-------|-------|------------------|-------------|----------|
| **metasfresh-4.9.8b** | 231,410 | 562,067 | **~6 s** | ~125 ms | ~5.7 s |
| **gbuilder** | 3,253 | 7,267 | **~12 ms** | ~4 ms | ~6 ms |

Harmonic (HyperBall) dominates on large graphs; betweenness stays sub-second at 230k nodes
because k=512 is fixed.

---

## Profiling commands

```bash
# Stage timings + centrality sub-phases
RUST_LOG=info,profile=info rgctl discover . -v 2>&1 | tee discover-profile.log
grep '\[profile\]' discover-profile.log
```

Lines to watch:

- `[profile] discover summary` — wall time, peak RSS, node count
- `[profile] stage` — index, centrality, save_analysis, save_dashboard, …
- `[profile] centrality breakdown` — pagerank / betweenness / harmonic seconds
- `[profile] centrality sub-phase` — percent of centrality wall per sub-phase

---

## Tests

| Test | Location | Purpose |
|------|----------|---------|
| HLL merge cardinality | `centrality_approx::tests` | Sketch correctness |
| Adaptive HyperBall gating | `centrality_approx::tests` | 500k cap → 8 rounds |
| Sampled bridge ranking | `centrality_approx::tests` | Bridge node scores high |
| HyperBall line graph | `centrality_approx::tests` | Head > tail harmonic |
| Columnar vs report | `centrality::tests` | `analyze_columnar` matches `analyze_with_view` |
| 10k / 50k mock budget | `centrality_approx_scale` | Scale gates |

```bash
cargo test --release -p rgbuilder-analysis centrality
cargo test --release --test centrality_approx_scale -- --nocapture
```

---

## rgbuilder-graph-perf (Aug 2026)

Phases 1–2 and most of phase 3 landed in `crates/rgbuilder-graph`:

- **Adjacency indexes** on `MemoryBackend` (`outgoing_adj` / `incoming_adj`) for O(degree) edge lookup.
- **Query pipeline** unified to `execute_node_ids` → `get_nodes_by_ids`; property filters use `for_each_node`.
- **Columnar I/O**: string-pool dedup, bulk row memcpy, lazy `id_to_index` (`OnceLock`), pre-sized snapshot buffers.
- **Compaction**: O(1) invalidated-path lookup via normalized `HashSet`.

Deferred follow-ups: none (phase 3 complete).

Profile results: `openspec/changes/rgbuilder-graph-perf/PROFILE.md`.

Bench: `cargo bench -p rgbuilder-graph --bench columnar_snapshot`.

---

## References

- Brandes, *A Faster Algorithm for Betweenness Centrality* (2001)
- Eppstein & Wang, *Approximating Betweenness Centrality* (2004)
- Boldi & Vigna, *HyperANF: Approximating the Neighborhood Function* (2013)
- Flajolet et al., *HyperLogLog* (2007)
