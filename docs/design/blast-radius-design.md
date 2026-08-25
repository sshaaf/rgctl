# Blast Radius — Engineering Design

Pre-computed **call-graph reachability** for change-impact analysis: upstream callers, impact scores, optional policy gates, and sub-second queries on large repos via mmap snapshots and a blast lookup cache.

![Blast Radius tab — impact metrics and caller table (gbuilder)](../images/design/blast-radius/blast-radius-impact.png)

*Figure 1: Dashboard **Blast Radius** tab — depth slider, impact score cards, and transitive caller table for a selected function.*

---

## 1. Goals

| Goal | How |
|------|-----|
| Answer “what breaks if I change this?” | Reverse call-graph traversal with SCC-aware macro impact |
| Stay fast at scale | T0 lookup cache → T1 mmap engine → optional socket daemon |
| Agent-ready output | JSON schema v2 (`target`, `metrics`, `topology`, `gatekeeping`) |
| Governance | Optional `--policy-file` centrality / cascade checks |

---

## 2. Architecture overview

```mermaid
flowchart TB
  subgraph discover["discover"]
    G[graph.snapshot.bin]
    E[blast_engine.snapshot.bin]
    C[macro_call_index.db]
    G --> E
    G --> C
  end

  subgraph cli["blast-radius SYMBOL"]
    T0[T0: MacroCallLookupDb]
    T1[T1: mmap engine + snapshot store]
    T3[T3: full graph + slices]
    T0 --> T1 --> T3
  end

  subgraph dash["Dashboard Blast tab"]
    WASM[WASM blastRadius BFS]
    BI[blast_index.json]
    BV[BlastView.tsx]
    BI --> BV
    WASM --> BV
  end

  discover --> cli
  E --> WASM
```

**Query tiers** (`src/cli/blast_radius.rs`): T0 blast lookup cache hit → optional `serve --daemon` socket → lite mmap path → full hydrate for `--with-slices` / `--policy-file`.

---

## 3. Scoring and topology

- **Impact score** (0–100): macro SCC impact from pre-built `BlastRadiusEngine`
- **Direct callers**: immediate incoming `Calls` edges
- **Impact zone**: transitive upstream callers; capped by `--depth N`
- **Canonical identity**: `target.canonical_fqn` + UUIDs (not display `fqn` alone)

CLI JSON: [json-api.md](../json-api.md) (blast-radius + field catalogs).

---

## 4. Rust implementation map

| Component | Path |
|-----------|------|
| Engine + reachability | `crates/rgbuilder-analysis/src/blast_radius_scc.rs` |
| Engine snapshot | `crates/rgbuilder-analysis/src/blast_engine_snapshot.rs` |
| T0 lookup cache | `crates/rgbuilder-analysis/src/macro_call_lookup.rs` |
| CLI orchestration | `src/cli/blast_radius.rs` |
| Socket daemon | `src/cli/query_daemon.rs` (`serve --daemon`) |
| Policy integration | `src/cli/policy_file.rs`, `engine.analyze_with_policy` |

---

## 5. Dashboard implementation

| Piece | Path |
|-------|------|
| Tab | `dashboard/src/BlastView.tsx` |
| WASM API | `blastRadius(nodeIndex, maxDepth)` in worker |
| Precomputed scores | `blast_index.json` (optional sort in sidebar) |
| Depth slider | Debounced re-query against WASM BFS |

---

## 6. CLI usage

```bash
rgctl discover .
rgctl -f json blast-radius ShoppingCartService
rgctl -f json blast-radius process --class OrderService --depth 3
rgctl -f json blast-radius Foo --policy-file policy.json   # exit 1 if VIOLATED
rgctl serve --daemon   # optional blast socket warm path
```

---

## 7. Testing

| Layer | Location |
|-------|----------|
| Release perf gates | `tests/blast_radius_perf.rs` |
| Subprocess golden path | `tests/cli_output/subprocess_golden_path.rs` |
| JSON contract | `tests/cli_output/all_commands_sanity.rs` |
| Dashboard harness | `tests/dashboard_harness.rs` (`blast_index.json`) |

Regenerate screenshots:

```bash
rgctl -r ~/git/java/gbuilder serve --port 8080
DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-design-screenshots.mjs
```

---

## 8. Related docs

- [CI policy checks design](ci-policy-checks-design.md) — `check` and `--policy-file`
- [Graph storage architecture](../graph-storage-architecture.md) — snapshots and blast cache
- [CLI I/O sanity QE](../cli-io-sanity-qe.md) — automated gates
