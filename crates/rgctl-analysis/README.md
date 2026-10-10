# rgctl-analysis

Graph analysis algorithms for [rgctl](https://github.com/sshaaf/rgctl): centrality, community detection, blast radius, CFG/PDG construction, slicing, taint analysis, and exact/fragment clone detection (SESE hammocks + 1-WL canonical hashing).

See [docs/analysis-architecture.md](../../docs/analysis-architecture.md) for the three-tier graph model.

## Module index

| Module | Graph input | Algorithm | Complexity |
|--------|-------------|-----------|------------|
| `graph_utils` | Backend / snapshot | Topology projection | O(V+E) |
| `callgraph` | Backend | u32 adjacency build | O(V+E) |
| `centrality` | `PetGraphView` | PageRank, Brandes / sampled betweenness, columnar discover path | O(k·E) PageRank; approximate BC/Harm |
| `centrality_approx` | `FlatGraphIndex` | Sampled Brandes, parallel HyperBall HLL | Approximate; dominates harmonic on large V |
| `community` | `PetGraphView` | Label propagation + modularity | O(iters·E) |
| `blast_radius_scc` | Call DiGraph | Kosaraju + bitset reachability | O(1) query |
| `blast_radius` | `PetGraphView` | Reverse BFS | O(V+E) |
| `dependency` | `PetGraphView` | Kosaraju SCC, reverse BFS | O(V+E) |
| `complexity` | Backend properties | Aggregation | O(F) |
| `cfg_builder` | tree-sitter AST | CFG construction | O(stmts) |
| `dominance` | CFG | Cooper-Harvey-Kennedy idom + Lengauer-Tarjan post-dominators | O(n²) idom / O(V+E) pdom |
| `sese` | CFG | Single-Entry Single-Exit (SESE) hammock decomposition ($3 \le |S| \le 15$) | O(V+E) |
| `wl_hash` | CFG SESE hammocks | 2-iteration Weisfeiler-Lehman (1-WL) canonical color refinement | O(V+E) |
| `clones` | Function nodes | Exact Type-1 (`code_hash`) grouping & Type-2 bloom LSH bands | O(N) exact; O(bands·B²) bloom |
| `fragment_clones` | Snapshot + CFGs | Two-stage candidate filter: bitwise bloom pre-filter + 1-WL hammock match | O(N) bitwise Stage 1 + O(cand·(V+E)) Stage 2 |
| `dataflow` | CFG + PDG | Reaching definitions | O(n·d) |
| `pdg` | CFG | Data + control dependencies | O(n·d) |
| `slicing` | PDG | Backward BFS slice | O(V+E) |
| `taint` | PDG | Forward taint propagation | O(V+E) |
| `migration` | Community graph | Weighted topo sort | O(V+E) |
| `results` | — | Columnar metric storage | O(1) lookup |
| `semantic_search` | Function nodes | Binary Hamming index + query | O(n) scan |
| `semantic_vocab` | Tokens | Compiled vocab accumulate (`vocab-accumulate-v1`) | O(tokens·D) |
| `semantic_diffuse` | `CallGraph` + dense `f32` | Jacobi neighbor blend before quantize | O(K·E·D) |
| `semantic_fusion` | Index + `AnalysisResults` | Late fusion re-rank | O(pool) |

Semantic search is **opt-in** (`rgctl semantic index`, default vocab). Use `--embedder code-daemon` for ONNX. Profile queries with `--release`; see [semantic-search-design.md](../../docs/design/semantic-search-design.md).

## Community detection naming

rgctl does **not** run the Leiden algorithm today. What ships is **label propagation** ([Raghavan et al., 2007](https://doi.org/10.1103/PhysRevE.76.036106)) with Newman modularity scoring, plus hub stripping and deterministic tie-breaking. Docs/UI still say “Louvain” in places (`louvain_community_id`, migration layout), and [`.github/TASK_PLAN.md`](../../.github/TASK_PLAN.md) lists Leiden as planned but unimplemented.

| Name in repo | What it actually is |
|--------------|---------------------|
| `CommunityDetector` | Label propagation on `Calls` + `Uses` |
| “Louvain” in dashboard/migration | Majority vote of label-propagation ids |
| Leiden (task 2.1.1) | Not implemented |

See also [graph-metrics-design.md](../../docs/design/graph-metrics-design.md#31-community-detection-naming).

## Running tests and benchmarks

```bash
cargo test -p rgctl-analysis
cargo clippy -p rgctl-analysis -- -D warnings
cargo bench --bench graph_benchmarks      # PetGraphView + blast radius
cargo bench --bench centrality_benchmarks   # PageRank, betweenness
cargo bench --bench community_benchmarks    # Label propagation
```

## Conventions

- Build `PetGraphView` once per analysis pass; pass references to analyzers.
- Use `TraversalConfig` (default depth 10) for bounded BFS traversals.
- Prefer `BlastRadiusEngine` over `BlastRadiusAnalyzer` for large graphs needing full transitive closure.
- Discover uses **`analyze_columnar`** for centrality; profile with `RUST_LOG=profile=info discover -v`.
- No `unwrap()` in production paths; propagate errors with `rgctl_error::Result`.
- `Node` string-like fields (`name`, `qualified_name`, `file_path`) use `SharedStr`; in tests prefer passing `&str` directly to builders (e.g. `Node::new(..., "name")`) and only call `.into()` when assigning owned `String` values into `Option<SharedStr>` fields.
