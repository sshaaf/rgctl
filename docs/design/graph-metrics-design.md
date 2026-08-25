# Graph Metrics — Engineering Design

**Network analytics** on the live call graph: PageRank, betweenness centrality, and community detection — computed at `discover` and surfaced in the **Functions** tab and migration planner.

![Functions tab — centrality columns (gbuilder)](../images/design/graph-metrics/graph-metrics-functions-table.png)

*Figure 1: **Functions** tab — sortable PageRank (PR), betweenness (BC), harmonic (Harm), and blast columns over the full function inventory.*

---

## 1. Goals

| Goal | How |
|------|-----|
| Find architectural hotspots | PageRank + betweenness on call graph |
| Migration batching | Communities (label propagation) + harmonic centrality |
| Agent ranking | `-f json metrics --pagerank` |
| Dashboard sort/filter | WASM-paginated function table |

---

## 2. Architecture overview

```mermaid
flowchart TB
  subgraph discover["discover"]
    G[Call graph]
    AR[analysis_results.bin]
    FM[function_metrics.json]
    G --> AR
    AR --> FM
  end

  subgraph metrics["Metrics commands"]
    PR[PageRank]
    BC[Betweenness]
    COM[Communities]
    CLI[rgctl metrics]
    PR --> CLI
    BC --> CLI
    COM --> CLI
  end

  subgraph ui["Dashboard"]
    FV[FunctionsView.tsx]
    MV[MigrationView.tsx]
    FM --> FV
    AR --> MV
  end
```

---

## 3. Metrics reference

| Metric | Meaning | Used in |
|--------|---------|---------|
| **PageRank** | Global importance in call graph | Functions tab, migration α term |
| **Betweenness** | Bridge / bottleneck score | Functions tab, policy cascade hazard |
| **Harmonic** | Reachability closeness | Functions tab, migration β term |
| **Communities** | Label-propagation clusters | Graph colors, migration Louvain vote |
| **Blast score** | Precomputed impact (per function) | Functions tab, migration γ term |

Background: [harmonic-centrality.md](../harmonic-centrality.md), [migration-algorithms.md](../migration-algorithms.md), [internal/temp.md](../internal/temp.md) (approximate algorithms + kernel-scale timings).

---

## 3.2 Large-graph behavior (≥ 50k nodes)

Discover applies **adaptive centrality gating** and a **columnar write path** so kernel-scale repos stay within memory and time budgets.

| Concern | Behavior |
|---------|----------|
| Centrality storage | Flat `Vec` → `AnalysisResults::CentralityTable` (no UUID `HashMap` on discover) |
| PageRank (V > 500k) | 8 iterations, ε=1e-4 |
| HyperBall harmonic (V > 500k) | 8 rounds, **Rayon-parallel** node scatter |
| `function_metrics.json` | **`sparse_mode: "community_only"`** — empty rows; use WASM + metagraph |
| `metagraph.json` | Package-level aggregates only (`member_indices` omitted at scale) |
| Dashboard export | In-memory `AnalysisResults` via `DashboardExportContext` (no reload per stage) |

**Profiling:** `RUST_LOG=profile=info rgctl discover . -v` → `[profile] centrality sub-phase` lines.

**CLI precision:** `rgctl metrics --pagerank --iterations N` still honors explicit iteration counts on demand.

---

## 3.1 Community detection naming

**Naming first:** rgBuilder does **not** run the Leiden algorithm today. What ships is **label propagation** ([Raghavan et al., 2007](https://doi.org/10.1103/PhysRevE.76.036106)) with Newman modularity scoring, plus hub stripping and deterministic tie-breaking. Docs and UI still say “Louvain” in places (`louvain_community_id`, migration layout colors), and [`.github/TASK_PLAN.md`](../../.github/TASK_PLAN.md) lists Leiden as planned but unimplemented.

| Name in repo | What it actually is |
|--------------|---------------------|
| `CommunityDetector` | Label propagation on `Calls` + `Uses` |
| “Louvain” in dashboard / migration | Majority vote of label-propagation ids (layout color only) |
| Leiden (task 2.1.1) | **Not implemented** |

Implementation: [`crates/rgbuilder-analysis/src/community.rs`](../../crates/rgbuilder-analysis/src/community.rs). For migration batching, the dashboard uses **package/module macro nodes**; community ids mainly drive graph coloring and cluster-aware layout, not the primary schedule order.

---

## 4. Rust implementation map

| Component | Path |
|-----------|------|
| Centrality | `crates/rgbuilder-analysis/src/centrality.rs`, `centrality_approx.rs` |
| Communities | `crates/rgbuilder-analysis/src/community.rs` |
| Harmonic | `crates/rgbuilder-analysis/src/centrality_approx.rs` (`HyperBallHarmonic`) |
| Persist | `crates/rgbuilder-analysis/src/results.rs` |
| CLI | `src/cli/metrics.rs` |
| Dashboard export | `crates/rgbuilder-dashboard/src/export_context.rs`, `function_metrics_export.rs` |

---

## 5. Dashboard implementation

| Piece | Path |
|-------|------|
| Tab | `dashboard/src/FunctionsView.tsx` |
| Data | WASM `list_nodes` + `function_metrics.json` merge |
| Sort | Column headers PR / BC / Harm / Blast |
| Tooltips | `FUNCTION_COLUMN_TOOLTIPS` in `functionListUtils.ts` |

Graph tab uses `communities.json` / metagraph community colors for package view (label-propagation ids; see [§3.1 Community detection naming](#31-community-detection-naming)).

---

## 6. CLI usage

```bash
rgctl discover .
rgctl metrics
rgctl -f json metrics --pagerank --iterations 50
rgctl -f json metrics --betweenness
rgctl -f json metrics --communities
```

`discover` already computes core metrics; `metrics` re-emits them as JSON without re-indexing.

---

## 7. Testing

| Layer | Location |
|-------|----------|
| Analysis unit tests | `crates/rgbuilder-analysis/src/centrality.rs`, `community.rs` |
| CLI subprocess | `tests/cli_output/all_commands_sanity.rs` |
| Dashboard harness | `tests/dashboard_harness.rs` (`function_metrics.json`) |

Screenshots: `capture-design-screenshots.mjs` → `docs/images/design/graph-metrics/`.

---

## 8. Related docs

- [Migration planner design](migration-planner-design.md)
- [Blast radius design](blast-radius-design.md)
- [GQL design](gql-design.md)
