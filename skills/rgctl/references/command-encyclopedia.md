# Command Encyclopedia

Detailed reference for all rgctl commands with full JSON samples and field specifications.

Samples below are truncated where noted. Field names match live CLI / `docs/json-api.md`. Fixture: `rgctl-tests/ecommerce-java` unless noted **illustrative** (schema-faithful shape).

## Table of Contents

- [discover](#discover)
- [gql](#gql)
- [blast-radius](#blast-radius)
- [slice](#slice)
- [inspect](#inspect)
- [metrics](#metrics)
- [semantic](#semantic)
- [communities](#communities)
- [cpg](#cpg)
- [check](#check)
- [export](#export)
- [serve](#serve)

---

## discover

**Command:** `rgctl [-f json] discover [PATH] [-l/--languages CSV] [-e/--exclude GLOB] [-v/--verbose] [--with-cfg] [--with-security] [--with-taint] [--with-dashboard] [--with-harmonic] [--export-migration-hints] [--with-ast-skeleton] [--with-dfg-loops] [--write-json-graph] …`

**Purpose:** Index the repo once (or after large changes). Build the graph agents query.

**Prerequisites:** None (this creates `.rgctl/`).

**Other flags:** `--languages java,go` restricts the language set; `--exclude` filters paths (glob); `--verbose` prints per-file progress (noisy — skip unless debugging a stuck/slow discover); `--write-json-graph` also writes legacy `graph.db`/`graph.json` (rarely needed, snapshot-only is the default and is what agents should rely on).

**Sample** (`-f json`, ecommerce-java):

```json
{
  "schema_version": 2,
  "command": "discover",
  "metrics": {
    "files_discovered": 66,
    "files_indexed": 66,
    "files_skipped": 0,
    "nodes_generated": 843,
    "edges_generated": 1793,
    "duration_ms": 306
  }
}
```

**Pitfalls:** Do not re-run on every question if `.rgctl/` exists. `--with-cfg` needed for slice/inspect/cpg PDG. `--with-taint` is discover-time taint (on-demand: `slice --taint`). Semantic search needs a separate `semantic index`.

**Agent should report:** files indexed, nodes/edges, duration; note which feature flags were used.

---

## gql

**Command:** `rgctl -f json gql '<MATCH…>'` or `rgctl -f json gql --macro-name <NAME> unused`

**Purpose:** Inventory, callers/callees, communities, path/relationship queries.

**Prerequisites:** `discover` done. Virtual `:Community` needs analysis overlay from discover.

**Sample** (macro `all_functions`):

```json
{
  "schema_version": 1,
  "count": 260,
  "rows": [
    [{ "binding": "f", "node": "addItem", "type": "Function",
       "file": "…/controller/CartController.java" }]
  ],
  "explain": false
}
```

**Useful patterns:**

```bash
# Incoming callers of X
rgctl -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = 'checkout' RETURN a,b LIMIT 20"
# Outgoing callees of X
rgctl -f json gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'checkout' RETURN a,b LIMIT 20"
# Name search (prefix or suffix only — *middle* silently returns 0)
rgctl -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
# Communities macro
rgctl -f json gql --macro-name all_communities unused
```

**Pitfalls:** `--macro-name` still needs a positional query arg — pass `unused`. `--explain` plan is text-mode only. rgctl GQL is a **subset of Cypher** — no `COUNT`, `ORDER BY`, `GROUP BY`, or aggregation functions. CALLS edges are static — interface / dynamic dispatch (receiver methods, virtual calls, trait impls) may not appear; if a `CALLS*1..N` query returns 0 edges for a method you know is called, fall back to `grep` for call sites. If LIKE on function names returns 0 for a concept (e.g. "ingress", "gateway"), it likely lives in package/directory names, type names, or community labels — try `communities list`, `semantic query`, or broaden the LIKE to non-Function node types before concluding nothing exists.

**Agent should report:** matching symbols, files, hop relationships — not raw row dumps.

---

## blast-radius

**Command:** `rgctl -f json blast-radius '<Symbol>' [--depth N] [--class C] [--file P] [--with-slices] [--policy-file PATH] [--no-policy]`

**Purpose:** Upstream change impact — who breaks if this symbol changes.

**Prerequisites:** `discover` done.

**Sample** (schema v2 shape; ecommerce names — field set matches live CLI):

```bash
rgctl -f json blast-radius 'checkout' --class OrderService --depth 3
```

```json
{
  "schema_version": 2,
  "target": {
    "id": "424d403b-1b2c-4a3d-8e9f-0c1b2a3f4e5d",
    "symbol": "checkout",
    "class_context": "OrderService",
    "file_path": "…/service/OrderService.java",
    "language": "java",
    "signature": "public OrderDto checkout() {",
    "canonical_fqn": "OrderService::checkout"
  },
  "metrics": {
    "score": 25.05,
    "direct_callers_count": 1,
    "impact_zone_size": 3,
    "caller_depth_limit": 3
  },
  "topology": {
    "scc_component_id": null,
    "direct_callers": [
      {
        "id": "8b2c4a3d-0c1b-4e5d-8e9f-424d403b1b2c",
        "fqn": "OrderController.checkout",
        "file_path": "…/OrderController.java"
      }
    ],
    "impact_zone": [
      {
        "id": "…",
        "fqn": "…",
        "file_path": "…"
      }
    ]
  },
  "gatekeeping": { "policy_status": "SKIPPED", "violations": [], "handoffs": [] }
}
```

**Pitfalls:** Ambiguous names need `--class` / `--file`. `--with-slices` is slow. Exit `1` when policy `VIOLATED` (JSON still emitted first). `--no-policy` skips policy evaluation entirely (gatekeeping reports `SKIPPED`) — use for pure impact analysis when the user isn't asking about CI gates. **Interface / dynamic dispatch:** blast-radius and CALLS edges track static call sites only — receiver methods, virtual calls, and trait/interface impls may return score=0 / 0 callers even when widely used. If blast-radius returns 0 for a method that clearly has callers, fall back to `grep` for call sites in source.

**Agent should report:** score, direct callers (`fqn` / `file_path`), impact_zone_size, policy status — not full topology arrays. Ignore or pass through extra v2 fields (`id`, `language`, `signature`, `scc_component_id`) as needed.

---

## slice

**Command:** `rgctl -f json slice <FILE> --line N --variable V [--function METHOD] [--direction backward|forward] [--taint] [--view text|cfg|pdg]`

**Purpose:** Line-level data dependence (what affects V / where V flows). `--taint` for source→sink security.

**Prerequisites:** Prefer `discover --with-cfg`. `--function` is the **method/function name**, not the class.

**Sample** (ecommerce-java `CartService.addItem`):

```bash
rgctl -f json slice src/main/java/com/example/ecommerce/service/CartService.java \
  --line 38 --variable cart --function addItem --direction backward
```

```json
{
  "schema_version": 1,
  "file": "src/main/java/com/example/ecommerce/service/CartService.java",
  "direction": "backward",
  "criterion": { "line": 38, "variable": "cart" },
  "lines": [38],
  "reduction_percent": 92.86,
  "nodes": [
    {
      "id": "node_0",
      "kind": "Expression",
      "label": "Cart cart = getUserCart();",
      "line": 38
    }
  ],
  "edges": []
}
```

**Pitfalls:** Wrong `--function` (class vs method) is a common failure. Needs CFG archive.

**Agent should report:** criterion, direction, `nodes[].label` / lines, reduction — not the full edge list unless asked.

---

## inspect

**Command:** `rgctl -f json inspect <SYMBOL> cfg [--prune] | pdg [--edge-layer all|data|control] [--def-use] | dom [--frontiers]`

**Purpose:** Raw CFG / PDG / dominator view for one function.

**Prerequisites:** `discover --with-cfg`. Symbol only — **no** `--class` (disambiguate via blast-radius / GQL first).

**Layer flags:** `cfg --prune` drops unreachable blocks before display. `pdg --edge-layer data|control` filters to one dependence type (default `all`); `--def-use` adds def-use variable lists per node. `dom --frontiers` prints dominance frontiers instead of just the tree.

**Sample:**

```json
{
  "schema_version": 1,
  "symbol": "addItem",
  "layer": "cfg",
  "pruned": false,
  "nodes": [
    {
      "id": "block_0",
      "block_index": 0,
      "start_line": 0,
      "end_line": 0,
      "statements": []
    },
    {
      "id": "block_1",
      "block_index": 1,
      "start_line": 24,
      "end_line": 24,
      "statements": [
        { "kind": "Return", "line": 24, "text": "return cartService.addItem(…);" }
      ]
    }
  ],
  "edges": [
    { "kind": "return", "source": "block_1", "target": "block_0" }
  ]
}
```

There are **no** `nodes_count` / `edges_count` fields — use `len(nodes)` / `len(edges)`.

**Pitfalls:** Ambiguous symbols fail; resolve FQN/name carefully. CFG may be **partial** on complex methods (covering only the first branch/entry block) — supplement with source reading if the block count seems low for the method's complexity.

**Agent should report:** layer, `len(nodes)` / `len(edges)`, notable `statements[].text` — not every node. Note if the CFG appears incomplete.

---

## metrics

**Command:** `rgctl -f json metrics [--pagerank] [--betweenness] [--communities] [--iterations N]`

**Purpose:** Hotspots (PageRank), bridges (betweenness), community stats.

**Prerequisites:** `discover` done. Default (no flags) computes all sections.

**Sample** (`--pagerank`; `top[]` entries are node UUIDs — **not names**):

```json
{
  "schema_version": 1,
  "pagerank": {
    "top": [{ "node": "<uuid>", "pagerank": 0.0117 }],
    "converged": true,
    "iterations": 20,
    "max_delta": 0.0
  }
}
```

**Resolving UUIDs to names:** PageRank covers all node types (Functions, Modules, Classes). To get the actual name/file for a UUID:

```bash
# For Function nodes (cheap, O(1)):
rgctl -f json cpg function '<uuid>'
# For any node type (heavier but always works):
rgctl -f json blast-radius '<uuid>'
```

Loop over `top[]` UUIDs and resolve each. GQL `WHERE n.id = '<uuid>'` does **not** work (node id is not a queryable property).

**Agent should report:** top hotspot symbols (resolve UUIDs first), modularity/community count when requested.

---

## semantic

**Command:**

```bash
rgctl semantic index [--embedder vocab|hash|onnx|code-daemon] [--embed-bodies] [--model PATH] [--tokenizer PATH] \
  [--dimensions N] [--incremental] [--diffuse] [--diffuse-alpha F] [--diffuse-iters N] [--diffuse-bidirectional]
rgctl semantic distill --matrix PATH [--embedder code-daemon|hash|onnx] [--tokens PATH] [--dimensions N]
rgctl -f json semantic query "…" [--limit N] [--scope function|community] \
  [--expand neighbors|blast|gql|all] [--expand-depth N] [--no-fusion] [--candidate-pool N] [--keyword-and]
```

**Purpose:** Natural-language / keyword find of functions (and community-scoped search), with optional one-shot expansion into graph context.

**Prerequisites:** `discover`, then **`semantic index`** (separate artifact). Default **vocab** (no ONNX). `--embedder onnx` needs `--model` (+ optional `--tokenizer` for SentencePiece). `--embedder code-daemon` needs ONNX weights (`git lfs pull`). `--embed-bodies` re-reads function source (off by default).

**Index tuning:** `--dimensions` (default 256, multiple of 8) trades index size for precision. `--incremental` (default true) reuses embeddings for unchanged `code_hash`. `--diffuse` blends each embedding toward its call-graph neighbors' mean (Jacobi iterations via `--diffuse-alpha`/`--diffuse-iters`; `--diffuse-bidirectional` includes callers, not just callees) — useful when bare-name/docstring signal is weak and callers/callees disambiguate intent; `--no-diffuse` forces it off.

**Query expansion:** `--expand neighbors` pulls CALLS neighbors of top hits, `--expand blast` runs blast-radius on top hits, `--expand gql` returns a ready GQL query, `--expand all` does all three — use when the user's NL query implies "and show me what's connected," so you skip a manual follow-up call. `--expand-depth` controls hop depth for `neighbors`/`gql` expansion (default 1). `--no-fusion` returns pure Hamming top-k (skip late-fusion re-ranking — rarely needed). `--candidate-pool` widens/narrows the pre-fusion candidate set (default 256). `--keyword-and` requires all query keywords to match entry metadata (stricter than default OR).

**Sample** (default vocab, query `checkout cart`):

```json
{
  "schema_version": 3,
  "query": "checkout cart",
  "model_id": "vocab-accumulate-v1",
  "dimensions": 256,
  "index_schema_version": 1,
  "hits": [
    {
      "name": "getCart",
      "qualified_name": "CartController.getCart",
      "node_id": "94823a58-9efd-4de4-95fb-aa082c2012c3",
      "score": 0.50,
      "fused_score": 0.50,
      "distance": 40,
      "ranking": "fusion",
      "file_path": "…/CartController.java"
    }
  ]
}
```

**Pitfalls:** Query without index fails. Restart `serve` after rebuilding index for dashboard search. **Large repos (100K+ nodes):** `--scope community` may return only singleton communities because label-propagation produces very granular clusters. For subsystem ownership on large repos, prefer `communities list` + grep labels over `--scope community`.

**Agent should report:** top hit names, files, scores (`score` / `fused_score`); keep `node_id` for follow-up GQL — not every hit.

---

## communities

**Command:** `rgctl -f json communities list` | `rgctl communities label [--write]`

**Purpose:** Named community overlay (subsystems). `label` recomputes heuristic labels (e.g. after renames shift what a cluster "is about") and persists them into `analysis_results.bin` (`--write` defaults to `true`; response's `written` field confirms).

**Prerequisites:** `discover` (community detection during analysis).

**Sample:**

```json
{
  "schema_version": 1,
  "modularity": 0.45,
  "written": false,
  "communities": [
    { "id": 462, "label": "ecommerce.service::findByEmail", "member_count": 19 }
  ]
}
```

**Agent should report:** top labels + sizes; use GQL `community_id` for members.

---

## cpg

**Command:** `rgctl -f json cpg <subcommand> …`

**Purpose:** Hybrid CPG façade (repo topology + CFG/PDG archive).

**Prerequisites:** `discover`; **`--with-cfg`** for PDG/slice/mutations/flows; `--with-ast-skeleton` for `ast`.

### cpg status

```bash
rgctl -f json cpg status
```

**Purpose:** Is the L_proc / CFG–PDG archive ready?

**Agent should report:** ready/not ready; whether to re-run `discover --with-cfg`.

### cpg function / cpg calls

```bash
rgctl -f json cpg function '<Symbol>'
rgctl -f json cpg calls '<Symbol>'
```

**Purpose:** Resolve a function in L_repo and whether L_proc exists; CALL neighborhood.

**Agent should report:** resolved identity + direct call neighbors.

### cpg pdg / cpg slice / cpg flows

```bash
rgctl -f json cpg pdg '<Symbol>' [--edge-layer all|data|control] [--def-use]
rgctl -f json cpg slice …   # wraps slice; see slice flags
rgctl -f json cpg flows FILE --line N --variable V --function F \
  [--direction forward|backward] [--with-alias]
```

**Purpose:** Dependence / data-flow overlays (prefer these when already in a CPG workflow).

**Pitfalls:** Missing archive → re-discover `--with-cfg`. `--with-alias` expands may-alias names. `--line` must point to a line **inside a function body** — struct definitions, import blocks, or other non-function lines will fail or return empty results.

**Agent should report:** key dependent statements / flow direction — not full graphs.

### cpg mutations

```bash
rgctl -f json cpg mutations --type ShoppingCart [--exclude-ctors] [--member fieldName] [--include-unresolved]
```

**Purpose:** Field mutations on a type (cart / DTO safety).

**Prerequisites:** `discover --with-cfg`.

**Other flags:** `--member` narrows to one field (e.g. "who writes `items`?" instead of the whole type). `--include-unresolved` also reports writes whose receiver type couldn't be statically resolved (dynamic dispatch) — noisier but catches mutations `blast-radius`/CALLS would miss.

**Agent should report:** which fields are written, by which functions.

### cpg ast

```bash
rgctl -f json cpg ast '<Symbol>'
```

**Purpose:** Coarse AST skeleton for a function.

**Prerequisites:** `discover --with-ast-skeleton`.

**Agent should report:** skeleton summary / notable nodes.

### cpg export

```bash
rgctl cpg export --format graphson --output cpg.json [--path-contains src/] \
  [--include-l-proc] [--include-field-writes]
```

**Purpose:** Export hybrid CPG view (GraphML / GraphSON).

**Other flags:** `--include-l-proc` and `--include-field-writes` both default to **on** (merging PDG DATA_FLOW edges and mutation-index field-write sites respectively) — there's no CLI switch to turn them off; they're primarily documentation of what a plain `cpg export` already includes.

**Agent should report:** output path + format.

**General cpg pitfalls:** Archive IO errors mean re-run `discover --with-cfg` (and ensure write permissions under `.rgctl/analysis/`).

---

## check

**Command:** `rgctl -f json check --policy-file policy.json`

**Purpose:** CI gate — fail when blast-radius policy rules are violated.

**Prerequisites:** `discover`; valid policy file (`docs/policy-format.md`).

**Sample:**

```json
{
  "schema_version": 1,
  "passed": true,
  "policy": "rgctl-tests/rgctl-policy.json",
  "violations": []
}
```

**Pitfalls:** Exit code `1` on failure — still parse JSON for violations.

**Agent should report:** passed/failed + violation summaries.

---

## export

**Command:** `rgctl export --export-format mermaid|graphviz|… --export-output OUT [--query FILTER]`

**Purpose:** Export graph / neighborhood diagrams.

**Prerequisites:** `discover` done.

**Pitfalls (critical):** `--query` uses **filter** syntax — `name:Foo`, `type:Function`, `all` — **not** GQL `MATCH … RETURN`. Agents must not pass MATCH strings to `--query`.

**Agent should report:** output path + format; confirm filter used.

---

## serve

**Command:** `rgctl serve [--open] [--host H] [--port N] [--dashboard-dir DIR] [--query-only|--dashboard-only] [--mode standard|mcp] [--daemon]`

**Purpose:** HTTP dashboard + `POST /api/query` (and semantic routes). **`--mode mcp`:** stdio MCP (seven tools, no HTTP). **`--daemon`:** foreground bootstrap of the background HTTP+MCP daemon (same model as `rgctl daemon start`; cache under `~/.rgctl/`).

**Prerequisites:** `discover` (dashboard bundle with `--with-dashboard` for full UI).

**Pitfalls:** `--daemon` is **not** the retired Unix-socket blast daemon — it starts the shared HTTP+MCP daemon. Do not combine `--daemon` with `--host`/`--open` on the same process (use `daemon start` for background HTTP). **`--idle-secs`** (default 300) applies to daemon idle shutdown.

**Agent should report:** URL/port for HTTP; note MCP stdio vs HTTP `/mcp` when relevant.

---

## See Also

- [JSON API Reference](../../docs/json-api.md) - Complete field specifications
- [Agent Recipes](../../docs/agent-recipes.md) - Copy-paste command examples
- [User Guide](../../docs/user-guide.md) - Full CLI reference
