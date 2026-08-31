# rgctl JSON API

Programmatic reference for parsing rgctl output. Every structured CLI command emits **one JSON document on stdout** when invoked with `-f json` / `--format json`.

**Canonical JSON reference** (includes field catalogs formerly in `cli-output-schemas.md`).

**Source of truth (Rust types):** `crates/rgctl-service` JSON modules (CLI `src/cli/*_output.rs` re-exports). MCP tools return the same `schema_version` payloads via `structuredContent`.

---

## Table of contents

1. [Invocation](#1-invocation)
2. [Schema versioning](#2-schema-versioning)
3. [Command index](#3-command-index)
4. [`discover`](#4-discover)
5. [`gql`](#5-gql)
6. [`blast-radius`](#6-blast-radius)
7. [`metrics`](#7-metrics)
8. [`check`](#8-check)
9. [`slice`](#9-slice)
10. [`inspect`](#10-inspect)
11. [`export` (file formats)](#11-export-file-formats)
12. [On-disk JSON after `discover`](#12-on-disk-json-after-discover)
13. [Exit codes](#13-exit-codes)
14. [Parsing recipes](#14-parsing-recipes)
15. [`semantic`](#15-semantic)
16. [`communities`](#16-communities)
17. [`cpg`](#17-cpg)
18. [`install`](#18-install)

---

## 1. Invocation

### Global flags

| Flag | Effect |
|------|--------|
| `-f json` | Emit structured JSON on **stdout** |
| `-r PATH` | Repository root (default: cwd) |
| `-o FILE` | Write stdout payload to a file instead of the terminal |

```bash
export REPO=/path/to/coolstore
rgctl -r "$REPO" -f json gql 'MATCH (n:Function) RETURN n LIMIT 5' | jq .
rgctl -r "$REPO" -f json blast-radius ShoppingCartService -o /tmp/blast.json
```

### Stdout vs stderr

| Mode | stdout | stderr |
|------|--------|--------|
| `-f json discover` | Single JSON telemetry object | Quiet (errors only) unless `-v` |
| `-f json` (other commands) | Single JSON result | Errors / warnings |
| Default text | Human-readable tables | Progress / info logs |

**Rule:** parse **stdout only** for JSON. Do not scrape stderr.

### MCP (`serve --mode mcp`)

Stdio JSON-RPC; no HTTP. Tools: `rgctl_status`, `rgctl_query`, `rgctl_search`, `rgctl_impact`, `rgctl_metrics`, `rgctl_cpg`, `rgctl_check`. Successful `tools/call` results use the same documents as CLI `-f json` (pretty text plus `structuredContent`).

`rgctl_query` and `rgctl_search` apply **`limit` 20** when the client omits `limit`. CLI `-f json gql` / `semantic query` do **not** add that default.

If the graph, CFG archive, or semantic index is missing, those tools return pipeline status (`command`: `pipeline_status`, `schema_version` 1) as the **tool result**, not a JSON-RPC error. `cpg export` is CLI-only (`rgctl_cpg` `op` `export` is unknown).

Resources: `rgctl://status`, `rgctl://manifest`, `rgctl://migration-plan`. Walkthrough: [MCP Server](guides/mcp-server.md).

### Prerequisites

All query commands require a prior successful `discover` (creates `.rgctl/graph.snapshot.bin` and related caches). See [user-guide.md](user-guide.md).

---

## 2. Schema versioning

Every JSON payload includes a top-level **`schema_version`** integer. Check it before parsing nested fields.

```javascript
const doc = JSON.parse(stdout);
if (doc.schema_version !== 2) {
  throw new Error(`unsupported blast-radius schema ${doc.schema_version}`);
}
```

| Command | Current `schema_version` | Breaking changes |
|---------|------------------------:|------------------|
| `discover` | **2** | v2 introduced structured `metrics` block |
| `blast-radius` | **2** | v2 added `target.language`, `target.canonical_fqn`, `metrics.caller_depth_limit` |
| `gql` | **1** | — |
| `metrics` | **1** | — |
| `check` | **1** | — |
| `slice` | **1** | — |
| `inspect` | **1** | — |
| `semantic index` | **2** | CLI index telemetry |
| `semantic query` | **3** | hits + optional expansion / fusion fields |
| `semantic distill` | **1** | RBVK matrix write (hash/code-daemon teacher) |
| `communities` | **1** | list / label |
| `cpg` (status / mutations / flows / …) | **1** | per-subcommand shapes |
| `install` | **1** | skill write report |

**Omitted vs null:** optional fields are **absent** when unset (not `null`), unless noted otherwise. Empty collections are usually `[]`, not omitted.

**Graph topology reuse:** `slice` (text view) and `inspect` (cfg/pdg) share node/edge shapes documented in the field catalogs below (§ Field catalogs / inspect & slice).

---

## 3. Command index

| Command | `-f json` | Primary keys | Typical use |
|---------|:---------:|--------------|-------------|
| `discover` | ✅ | `metrics` | CI ingestion gates, timing |
| `gql` | ✅ | `rows`, `count` | Graph queries, inventory |
| `blast-radius` | ✅ | `target`, `metrics`, `topology` | Change-impact automation |
| `metrics` | ✅ | `pagerank`, `betweenness`, `communities` | Hotspot ranking |
| `check` | ✅ | `passed`, `violations` | CI policy gate |
| `slice` | ✅ | `lines` / `nodes` / `edges` / `taint` | Line-level analysis |
| `inspect` | ✅ | `layer`, `nodes`, `edges` | CFG/PDG/dominance dumps |
| `semantic` | ✅ | `hits` / `functions_indexed` | Opt-in NL / keyword search |
| `communities` | ✅ | `communities`, `modularity` | Named community labels |
| `cpg` | ✅ | varies by subcommand | Hybrid CPG façade |
| `install` | ✅ | `writes` | Install bundled agent skill |
| `export` | ❌ (file) | — | Full-graph serialization |
| `serve` | ❌ | — | HTTP dashboard + `/api/query` (default); `--mode mcp` stdio; `--daemon` background HTTP+MCP daemon bootstrap |

---

## 4. `discover`

```bash
rgctl -f json discover PATH [-l LANGS] [-e PATTERNS] [--with-cfg] [--with-taint] [--with-kantra] [--kantra-target NAME] [--full]
```

Kantra flags (`--with-kantra`, `--kantra-rules`, `--kantra-catalog`, `--kantra-target`, `--kantra-index-only`) do not change stdout JSON shape; they add `.rgctl/kantra_findings.json` and `KantraRule` nodes in `graph.snapshot.bin`. See [`kantra_findings.json`](#kantra_findingsjson).

### TypeScript shape

```typescript
interface DiscoverResponse {
  schema_version: 2;
  command: "discover";
  metrics: {
    files_discovered: number;
    files_indexed: number;
    files_skipped: number;
    nodes_generated: number;
    edges_generated: number;
    duration_ms: number;
  };
  full?: true;
  plan?: { id: "basic_discover" | "deep_pass" | "semantic_index"; status: string }[];
}
```

With `--full`, stdout is still **one** JSON object (`full: true` + `plan`). Live stage updates go to `.rgctl/pipeline_status.json` (`schema_version` 1, `command: "pipeline_status"`). `GET /api/status` on HTTP `serve` and MCP `rgctl_status` (`structuredContent`) return the same document. See [MCP Server](guides/mcp-server.md).

### Example

```json
{
  "schema_version": 2,
  "command": "discover",
  "metrics": {
    "files_discovered": 120,
    "files_indexed": 118,
    "files_skipped": 2,
    "nodes_generated": 1842,
    "edges_generated": 4103,
    "duration_ms": 12500
  }
}
```

### jq

```bash
rgctl -f json discover . | jq '.metrics | {nodes: .nodes_generated, ms: .duration_ms}'
```

---

## 5. `gql`

```bash
rgctl -f json gql "<QUERY>" [--macro-name NAME] [--explain]
```

CLI JSON does not default-limit rows. MCP `rgctl_query` applies `limit` 20 when omitted.

### TypeScript shape

```typescript
interface GqlResponse {
  schema_version: 1;
  rows: GqlRow[];       // one entry per MATCH result row
  count: number;        // always rows.length
  explain: boolean;     // mirrors --explain (plan is text-only)
}

interface GqlRow {
  binding: string;      // variable name from MATCH (e.g. "n", "a")
  node: string;         // bare symbol name (or community label)
  type: string;         // node type label, e.g. "Function" or "Community"
  file: string | null;  // source path when indexed
  community_id?: number; // present on :Community rows; optional on functions when joined
  label?: string;        // :Community label
  member_count?: number; // :Community size
}
```

Each `rows[i]` is an **array** of bindings (one object per variable in the `RETURN` clause).

Virtual `:Community` nodes and `f.community_id` filters join `.rgctl/analysis_results.bin`
(see [community-query-and-naming-plan.md](design/community-query-and-naming-plan.md)).

### Example

```json
{
  "schema_version": 1,
  "rows": [
    [
      {
        "binding": "n",
        "node": "ShoppingCartService",
        "type": "Function",
        "file": "src/main/java/com/redhat/coolstore/service/ShoppingCartService.java"
      }
    ]
  ],
  "count": 1,
  "explain": false
}
```

### jq

```bash
# All function names
rgctl -f json gql 'MATCH (n:Function) RETURN n' \
  | jq -r '.rows[][].node'

# Multi-binding row (a,b) from a CALLS query
rgctl -f json gql 'MATCH (a:Function)-[:CALLS]->(b:Function) RETURN a,b LIMIT 5' \
  | jq '.rows[] | map({binding, node, file})'

# Named communities
rgctl -f json gql --macro-name all_communities unused \
  | jq '.rows[:5][][] | {id: .community_id, label, member_count}'
```

### Macros

When `--macro-name` is set, the positional query string is ignored:

```bash
rgctl -f json gql --macro-name all_functions 'unused'
# Macros: all_functions | direct_calls | call_chain | all_communities
```

---

## 6. `blast-radius`

```bash
rgctl -f json blast-radius SYMBOL [--depth N] [--policy-file PATH] [--with-slices]
```

### TypeScript shape

```typescript
interface BlastRadiusResponse {
  schema_version: 2;
  target: {
    id: string;              // UUID
    symbol: string;
    class_context: string | null;
    file_path: string;
    language: string;        // "java" | "rust" | "python" | "unknown"
    signature?: string;      // omitted when unknown
    canonical_fqn: string;   // prefer for routing: "Class::method"
  };
  metrics: {
    score: number;           // 0–100
    direct_callers_count: number;
    impact_zone_size: number;
    caller_depth_limit?: number;  // present when --depth N
  };
  topology: {
    scc_component_id: number | null;
    direct_callers: SymbolContext[];
    impact_zone: SymbolContext[];
  };
  gatekeeping: {
    policy_status: "SKIPPED" | "PASS" | "VIOLATED";
    violations: PolicyViolation[];
    handoffs: SliceHandoff[];  // [] unless --with-slices
  };
}

interface SymbolContext {
  id: string;       // UUID — stable join key
  fqn: string;      // display name (language-native)
  file_path: string;
}

interface SliceHandoff {
  callee: string;
  param: string;
  index: number;
}
```

### Policy violations (`gatekeeping.violations`)

Tagged union — discriminant field is **`kind`**:

| `kind` | Fields |
|--------|--------|
| `domain_isolation` | `source_domain`, `reached_domain`, `node` |
| `scale_failure` | `count`, `max` |
| `cascade_hazard` | `node`, `betweenness`, `threshold` |
| `sanitization_bypass` | `sink_line`, `path_trace`, `sanitizer_node` |

### jq

```bash
# Impact score and caller UUIDs
rgctl -f json blast-radius ShoppingCartService \
  | jq '{score: .metrics.score, callers: [.topology.direct_callers[].id]}'

# Depth-capped impact zone
rgctl -f json blast-radius CartEndpoint --depth 3 \
  | jq '.metrics.caller_depth_limit, .topology.impact_zone | length'

# Policy gate
rgctl -f json blast-radius OrderService --policy-file policy.json \
  | jq '.gatekeeping.policy_status, .gatekeeping.violations'
```

**Routing rule:** use `target.canonical_fqn` and `topology.*.id` (UUID). Treat `topology.*.fqn` as display text only.

### Migration from legacy flat JSON

Older rgctl emitted a flat object (`symbol`, `score`, `direct_callers[]`, `impact_zone[]` at the root). Current output is **nested** with `schema_version: 2`. See the blast-radius field catalog in this document for the full table and jq path mapping.

```bash
# Was: jq '.score'  →  Now:
jq '.metrics.score'

# Was: jq '.direct_callers[]'  →  Now:
jq '.topology.direct_callers[].fqn'

# Prefer for automation (v2):
jq '.target.canonical_fqn'
```

---

## 7. `metrics`

```bash
rgctl -f json metrics [--pagerank] [--betweenness] [--communities] [--iterations N]
```

Default (no section flags) includes **all three** sections. Requesting a single flag omits the others entirely.

### TypeScript shape

```typescript
interface MetricsResponse {
  schema_version: 1;
  pagerank?: {
    top: { node: string; pagerank: number }[];  // max 20
    converged: boolean;
    iterations: number;
    max_delta: number;
  };
  betweenness?: { node: string; score: number }[];  // max 20, top-level array
  communities?: {
    count: number;
    modularity: number;
    assignments: number;
  };
}
```

### jq

```bash
rgctl -f json metrics --pagerank | jq '.pagerank.top[:5]'
rgctl -f json metrics | jq '.communities.modularity'
```

---

## 8. `check`

```bash
rgctl -f json check --policy-file policy.json
```

Evaluates policy rules against **git-changed** functions (or all functions if git is unavailable).

### TypeScript shape

```typescript
interface CheckResponse {
  schema_version: 1;
  policy: string;           // path passed to --policy-file
  passed: boolean;
  violations: {
    symbol: string;
    error?: string;         // engine error (mutually exclusive with violation)
    violation?: string;     // human-readable policy text
  }[];
}
```

### jq

```bash
rgctl -f json check --policy-file policy.json | jq '{passed, count: (.violations | length)}'
```

---

## 9. `slice`

```bash
rgctl -f json slice FILE --line N --variable VAR [--function NAME] \
  [--view text|cfg|pdg] [--direction backward|forward] [--taint]
```

Response shape depends on **`--view`** and **`--taint`**.

### `--view text` (default)

```typescript
interface SliceTextResponse {
  schema_version: 1;
  file: string;
  criterion: { line: number; variable: string };
  direction: "backward" | "forward";
  reduction_percent: number;
  lines: number[];              // source lines in the slice
  nodes: PdgNode[];             // PDG subgraph
  edges: PdgEdge[];
}
```

### `--view cfg`

```typescript
interface SliceCfgResponse {
  schema_version: 1;
  file: string;
  function: string;
  view: "cfg";
  nodes: CfgBlockNode[];
  edges: CfgEdgeNode[];
}
```

### `--view pdg`

Same topology as inspect PDG (`view: "pdg"`).

### `--taint`

Flat summary (no graph topology):

```typescript
interface SliceTaintResponse {
  schema_version: 1;
  file: string;
  function: string;
  line: number;
  variable: string;
  taint: true;
  flows: number;
  vulnerable: number;
}
```

### Shared graph primitives

```typescript
interface PdgNode {
  id: string;       // "node_0", …
  line: number;
  label: string;
  kind: string;
  defined?: string[];
  used?: string[];
}

interface PdgEdge {
  source: string;
  target: string;
  kind: "data" | "control" | string;
  variable?: string;  // data deps only
}

interface CfgBlockNode {
  id: string;       // "block_0", …
  block_index: number;
  start_line: number;
  end_line: number;
  statements: { line: number; kind: string; text: string }[];
}

interface CfgEdgeNode {
  source: string;
  target: string;
  kind: string;   // "next", "iftrue", "iffalse", …
}
```

### jq

```bash
# Lines touched by backward slice
rgctl -f json slice src/.../Foo.java --line 42 --variable x --function Foo \
  | jq '.lines'

# Taint counts only
rgctl -f json slice src/.../Foo.java --line 10 --variable input --function Foo --taint \
  | jq '{flows, vulnerable}'
```

---

## 10. `inspect`

```bash
rgctl -f json inspect SYMBOL cfg|pdg|dom [layer options]
```

Requires `discover --with-cfg` for richest PDG/CFG data from the analysis archive.

### CFG layer

```typescript
interface InspectCfgResponse {
  schema_version: 1;
  symbol: string;
  layer: "cfg";
  pruned: boolean;
  nodes: CfgBlockNode[];
  edges: CfgEdgeNode[];
}
```

### PDG layer

```typescript
interface InspectPdgResponse {
  schema_version: 1;
  symbol: string;
  layer: "pdg";
  nodes: PdgNode[];
  edges: PdgEdge[];
  data_deps: number;
  control_deps: number;
}
```

### Dominance layer

```typescript
interface InspectDomResponse {
  schema_version: 1;
  symbol: string;
  layer: "dom";
  nodes: { block_index: number; start_line: number; end_line: number }[];
  idom: { block: number; immediate_dominator: number }[];
  frontiers?: { block: number; frontier_blocks: number[] }[];  // with --frontiers
}
```

Block references use integer **`block_index`** (sorted by `start_line`), not string ids.

### jq

```bash
rgctl -f json inspect ShoppingCartService pdg --edge-layer data \
  | jq '{data: .data_deps, nodes: [.nodes[] | {line, label}]}'
```

**Diagram formats:** `-f mermaid` and `-f graphviz` emit diagram **text** (not JSON) for cfg/dom layers.

---

## 11. `export` (file formats)

`export` writes to **`--export-output`**; stdout is a one-line summary (unless global `-o` redirects).

```bash
rgctl export --export-format json --export-output graph.json --query all
rgctl export --export-format mermaid --export-output clearCart.mmd --query 'name:clearCart'
```

| `--export-format` | File content |
|-------------------|--------------|
| `json` | Graph snapshot JSON (filtered when `--query` ≠ `all`) |
| `graphml` | GraphML XML |
| `graphviz` | DOT |
| `mermaid` | Mermaid flowchart |
| `obsidian` | Obsidian vault directory (one markdown note per doc heading; needs markdown `discover` + `content_store.bin`) |
| `okf` | Open Knowledge Foundation JSON entity bundle (doc headings + bodies) |

`obsidian` and `okf` export **doc heading modules** from the graph; use `--query all`. Output for `obsidian` is a **directory** (`--export-output "$REPO/vault"`), not a single file.

`--query` uses **filter syntax** (`all`, `name:Foo`, `type:Function`, `functions`) — not GQL `MATCH`. The summary line reports the filtered node/edge counts (or note count for Obsidian).

---

## 12. On-disk JSON after `discover`

These files are written under `.rgctl/` (and copied into `.rgctl/dashboard/` for the UI). They are **not** emitted on stdout but are stable inputs for custom tooling.

| Path | `schema_version` | Purpose |
|------|------------------:|---------|
| `dashboard/manifest.json` | 1 | Bundle metadata, phase flags, metric summary |
| `dashboard/metagraph.json` | 2 | Package-level graph for LOD UI |
| `dashboard/cfg_index.json` | 1 | CFG function catalog |
| `dashboard/slice_index.json` | 1 | Slice/PDG function catalog |
| `dashboard/dataflow_index.json` | 1 | Dataflow function catalog |
| `dashboard/taint_index.json` | 1 | Taint summary (`discover --with-cfg`) |
| `dashboard/taint/{uuid}.json` | 1 | Per-function taint flows |
| `dashboard/slice/{uuid}.json` | 1 | Per-function source + PDG bundle |
| `dashboard/cfg/{uuid}.json` | 1 | Per-function CFG preview |
| `kantra_findings.json` | 2 | Kantra violations + enrichment + skipped rules (`discover --with-kantra`) |
| `file_hashes.json` | — | Incremental discover state |
| `content_store.bin` | — | Blake3-keyed blob store for truncated markdown bodies / large files (`body_ref`, `blob_ref`) |

### `manifest.json` (excerpt)

```json
{
  "schema_version": 1,
  "phases": { "0": "complete", "4": "complete", "8": "pending" },
  "graph": {
    "payload_path": "graph_payload.bin",
    "payload_format": "columnar_v2",
    "node_count": 1842,
    "edge_count": 4103
  },
  "analysis": {
    "cfg_available": true,
    "taint_available": true,
    "taint_flow_count": 12,
    "taint_vulnerable_count": 3
  },
  "metrics": {
    "function_count": 412,
    "avg_complexity": 1.2
  }
}
```

### `taint_index.json`

```json
{
  "schema_version": 1,
  "available": true,
  "detail_dir": "taint",
  "function_count": 8,
  "total_flows": 12,
  "vulnerable_flows": 3,
  "functions": [
    {
      "function_id": "uuid",
      "name": "ShoppingCartService",
      "file_path": "src/main/java/.../ShoppingCartService.java",
      "flow_count": 2,
      "vulnerable_count": 1
    }
  ]
}
```

### `taint/{uuid}.json` flow entry

```json
{
  "id": 0,
  "variable": "userInput",
  "source_type": "HttpParameter",
  "sink_type": "SqlQuery",
  "severity": 10,
  "vulnerable": true,
  "sanitizers": [],
  "source_line": 42,
  "sink_line": 88,
  "source_text": "...",
  "sink_text": "...",
  "path_lines": [42, 55, 88],
  "path_statements": ["...", "...", "..."]
}
```

### `kantra_findings.json`

Written by `discover --with-kantra` (eval + enrich stages). Rule nodes are indexed into `graph.snapshot.bin` (`KantraRuleset`, `KantraRule`); `VIOLATES` edges link rules to resolved code nodes after enrich.

Incremental filecontent results are cached under `.rgctl/kantra_cache/` (content-hash keyed; invalidated on ruleset change).

```json
{
  "schema_version": 2,
  "command": "kantra_findings",
  "catalog_id": "stable-java@022bbd34b34eca53d04b6cb2b97b27e47fef479b",
  "ruleset": "embedded-stable-java",
  "target_filter": "quarkus",
  "evaluated_rules": 2656,
  "violations": [
    {
      "rule_id": "springboot-00001",
      "category": "mandatory",
      "file": "src/main/java/com/example/Foo.java",
      "line": 12,
      "message": "…",
      "matched_by": "java.referenced",
      "symbol": "org.springframework.stereotype.Service",
      "enrichment": {
        "node_id": "550e8400-e29b-41d4-a716-446655440000",
        "community_id": 12,
        "pagerank": 0.0042,
        "blast_radius_score": 18.5,
        "impact_zone_size": 47
      }
    }
  ],
  "skipped_rules": [
    {
      "rule_id": "some-xml-rule",
      "reason": "unsupported: builtin.xml"
    }
  ],
  "cache_hits": 120,
  "cache_misses": 3
}
```

| Field | Type | Notes |
|-------|------|-------|
| `schema_version` | `2` | Artifact version |
| `command` | `"kantra_findings"` | Discriminator |
| `catalog_id` | string? | Embedded: `stable-java@<rulesets-git-sha>`; fixture: `fixture@<hash>`; override paths use `dir@…` / tree id |
| `ruleset` | string | Display name from catalog |
| `target_filter` | string? | Set when `--kantra-target` is active |
| `evaluated_rules` | number | Rules in catalog before per-rule skip |
| `violations` | array | Matches with `rule_id`, `file`, `line`, `matched_by` (`filecontent`, `java.referenced`, …) |
| `violations[].symbol` | string? | Import or symbol name when resolved by referenced rules |
| `violations[].enrichment` | object? | Graph linkage + metrics (after blast engine build) |
| `violations[].enrichment.node_id` | string? | UUID of matched graph node (used for `VIOLATES` edges) |
| `violations[].enrichment.community_id` | number? | Louvain community |
| `violations[].enrichment.pagerank` | number? | PageRank centrality |
| `violations[].enrichment.blast_radius_score` | number? | Blast-radius score for the node |
| `violations[].enrichment.impact_zone_size` | number? | Transitive impact zone size |
| `skipped_rules` | array | Unsupported providers, invalid regex, or eval errors (`rule_id`, `reason`) |
| `cache_hits` | number? | Per-file cache hits (`builtin.filecontent` warmup); omitted when zero |
| `cache_misses` | number? | Stale cache entries re-evaluated; omitted when zero |

Query violation edges: `MATCH (r:KantraRule)-[:VIOLATES]->(n) RETURN r, n LIMIT 20`.

Fixture override (`--kantra-rules`) omits full Konveyor `catalog_id` unless the ruleset was compiled from the submodule.

Binary artifacts (`graph.snapshot.bin`, `graph_payload.bin`, `blast_engine.snapshot.bin`) use internal columnar formats — use CLI JSON or `export --export-format json` for portable graph access.

---

## 13. Exit codes

| Command | `0` | `1` |
|---------|-----|-----|
| `discover` | Success | Failure |
| `gql` | Success | Query/IO error |
| `blast-radius` | Success, or policy skipped | `--policy-file` + `policy_status == "VIOLATED"` (JSON still on stdout) |
| `check` | `passed == true` | `passed == false` |
| `slice` / `inspect` / `metrics` / `export` | Success | Error |
| `install` | Skill files written or unchanged | Missing `--skill`, dest differs without `--force` (`skipped_exists`; JSON still on stdout), or I/O error |

**CI pattern:** capture stdout first, then check `$?`.

```bash
out=$(rgctl -f json blast-radius Foo --policy-file policy.json) || ec=$?
echo "$out" | jq .
exit "${ec:-0}"
```

---

## 14. Parsing recipes

### Python

```python
import json, subprocess

def rgctl_json(repo: str, *args: str) -> dict:
    cmd = ["rgctl", "-r", repo, "-f", "json", *args]
    out = subprocess.check_output(cmd, text=True)
    return json.loads(out)

doc = rgctl_json("/path/to/coolstore", "blast-radius", "CartEndpoint")
assert doc["schema_version"] == 2
for caller in doc["topology"]["direct_callers"]:
    print(caller["id"], caller["fqn"])
```

### Node.js

```javascript
import { execFileSync } from "node:child_process";

function rgctlJson(repo, ...args) {
  const out = execFileSync("rgctl", ["-r", repo, "-f", "json", ...args], {
    encoding: "utf8",
  });
  return JSON.parse(out);
}

const gql = rgctlJson(process.env.REPO, "gql", "MATCH (n:Function) RETURN n");
const names = gql.rows.flat().map((b) => b.node);
```

### CI ingestion gate

```bash
metrics=$(rgctl -f json discover .)
nodes=$(echo "$metrics" | jq '.metrics.nodes_generated')
test "$nodes" -gt 100
```

### Chaining discover → query

```bash
rgctl -f json discover . | tee discover.json
rgctl -f json gql --macro-name all_functions x | jq '.count'
```

---

## 15. `semantic`

Opt-in embedding index + query. Types: `src/cli/semantic_output.rs`.

### `semantic index`

```bash
rgctl -r "$REPO" -f json semantic index
rgctl -r "$REPO" -f json semantic index --scope docs --embedder hash
# extras: --embedder code-daemon|hash   --embed-bodies
```

`--scope` on **index**: `function` (default), `docs` (`:Module` `kind=heading` + `kind=code_block`), or `all`. Doc embeddings read `body_text` or `content_store.bin` via `body_ref`. Function vectors use declaration metadata unless `--embed-bodies`.

```typescript
type SemanticIndexJsonResponse = {
  schema_version: 2;
  model_id: string;            // e.g. vocab-accumulate-v1; +bodies when --embed-bodies
  dimensions: number;          // default 256
  functions_indexed: number;   // entry count (doc sections when --scope docs; field name is legacy)
  path: string;                // .rgctl/semantic_index.bin
  graph_digest?: string;
  build_stats?: {
    total: number;
    reused: number;
    embedded: number;
    removed: number;
  };
};
```

Text mode prints `Indexed N functions` — same count as `functions_indexed` (not always functions when `--scope docs`).

```bash
rgctl -r "$REPO" -f json semantic index | jq '{model_id, dimensions, functions_indexed}'
```

### `semantic distill`

Teacher-embed `vocab_tokens.txt` into an RBVK blob. Copy the file to `crates/rgctl-analysis/assets/vocab_matrix.bin` and rebuild to compile `vocab-accumulate-v2`. Teacher cannot be `vocab` (that would distill the table from itself). Default teacher is `code-daemon`; `--embedder hash` is for tests / offline CI.

```bash
rgctl -r "$REPO" -f json semantic distill --matrix vocab_matrix.bin --embedder hash
rgctl -r "$REPO" semantic distill --matrix crates/rgctl-analysis/assets/vocab_matrix.bin --embedder code-daemon
```

```typescript
type SemanticDistillJsonResponse = {
  schema_version: 1;
  path: string;
  tokens: number;
  dimensions: number;
  teacher_model_id: string;
  compiled_model_id: string;  // vocab-accumulate-v2
};
```

### `semantic query`

```bash
rgctl -r "$REPO" -f json semantic query "checkout flow" --limit 10
rgctl -r "$REPO" -f json semantic query "checkout flow" --scope docs --limit 10
```

No `--embedder` on query — uses the model saved in `semantic_index.bin`. `--scope docs` on query does **not** filter hits (except `--scope community`); build the index with matching `--scope` first.

```typescript
type SemanticHitJson = {
  node_id: string;
  name: string;
  qualified_name?: string;
  file_path?: string;
  distance: number;            // Hamming
  score: number;
  fused_score?: number;
  ranking?: string;            // e.g. "fusion"
};

type SemanticQueryJsonResponse = {
  schema_version: 3;
  query: string;
  model_id: string;
  dimensions: number;
  hits: SemanticHitJson[];
  expansion?: object;          // optional query expansion payload
};
```

```bash
rgctl -r "$REPO" -f json semantic query "OrderService" --limit 5 \
  | jq '.hits[:5] | map({name, score, file_path})'
rgctl -r "$REPO" -f json semantic query "cart" --scope community --limit 5 \
  | jq '.hits[].name'
```

---

## 16. `communities`

List / refresh heuristic labels over label-propagation clusters. Types: `src/cli/communities.rs`.

```bash
rgctl -r "$REPO" -f json communities list
rgctl -r "$REPO" -f json communities label --write
```

```typescript
type CommunitiesJsonResponse = {
  schema_version: 1;
  modularity: number;
  written: boolean;            // true after `label --write`
  communities: Array<{
    id: number;
    label: string;
    member_count: number;
  }>;
};
```

```bash
rgctl -r "$REPO" -f json communities list | jq '.communities[:10]'
rgctl -r "$REPO" -f json communities list | jq '{modularity, n: (.communities|length)}'
```

GQL alternative: `--macro-name all_communities` (see User Guide §6).

---

## 17. `cpg`

Hybrid CPG façade (needs `discover --with-cfg`). Types: `crates/rgctl-analysis/src/cpg.rs` + `src/cli/cpg.rs`. All JSON payloads use `schema_version: 1`.

### `cpg status`

```bash
rgctl -r "$REPO" -f json cpg status
```

```typescript
type CpgStatus = {
  schema_version: 1;
  archive_path: string;
  archive_present: boolean;
  function_count: number;
  graph_digest?: string;
  field_write_index_present: boolean;
  field_write_count: number;
  ast_skeleton_present: boolean;
  ast_skeleton_count: number;
};
```

### `cpg mutations`

```bash
rgctl -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors
```

```typescript
type CpgMutationsResult = {
  schema_version: 1;
  type_name: string;
  exclude_ctors: boolean;
  member?: string;
  include_unresolved: boolean;
  mutations: Array<{
    file: string;
    line: number;
    code: string;
    member: string;
    function: string;
    is_constructor: boolean;
    receiver_local?: string;
    receiver_type?: string;
    kind: string;
  }>;
};
```

### Other subcommands

| Subcommand | Primary keys |
|------------|--------------|
| `cpg function <Symbol>` | `id`, `name`, `has_l_proc`, … |
| `cpg calls <Symbol>` | `edges[]` (`direction`, `name`, `id`) |
| `cpg flows …` | `steps[]` (data dependence walk) |
| `cpg export` | writes a **file** (`--format` / `--output`); not stdout JSON |

```bash
rgctl -r "$REPO" -f json cpg status | jq '{archive_present, function_count, field_write_count}'
rgctl -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors \
  | jq '.mutations | length'
rgctl -r "$REPO" -f json cpg calls priceShoppingCart | jq '.edges[:10]'
```

---

## 18. `install`

Copy the bundled rgctl agent skill into project skill directories. Does **not** require a prior `discover`. Types: `src/cli/install_output.rs`. `schema_version` is **1**.

```bash
rgctl -r "$REPO" -f json install --skill [--host all|claude|cursor] [--force]
```

```typescript
type InstallWriteStatus = "created" | "unchanged" | "overwritten" | "skipped_exists";

type InstallResponse = {
  schema_version: 1;
  command: "install";
  skill: "rgctl";
  repo: string; // absolute repository root
  force: boolean;
  writes: Array<{
    host: "claude" | "cursor";
    path: string; // absolute dest path
    status: InstallWriteStatus;
  }>;
};
```

Without `--skill` the process exits 1 and does not emit this payload. If any write is `skipped_exists`, JSON is still printed and the process exits 1.

```bash
rgctl -r "$REPO" -f json install --skill | jq '.writes[] | {host, status}'
```

---

## Verification

Schema fixtures are tested in CI:

```bash
cargo test --test cli_output --test subprocess_golden_path --test all_commands_sanity
```

See [cli-io-sanity-qe.md](cli-io-sanity-qe.md) for the full coverage matrix.

---

## Related

- [user-guide.md](user-guide.md) — install, ecommerce-java walkthrough (CoolStore dual API), CLI examples
- Field catalogs — exhaustive tables later in this document (formerly `cli-output-schemas.md`)
- [http-api.md](http-api.md) — `rgctl serve` and `/api/query`
- [cli-io-sanity-qe.md](cli-io-sanity-qe.md) — subprocess JSON contract and release perf gates

---

# Field catalogs (from former cli-output-schemas)

## Conventions matrix

| Convention | blast-radius | discover | gql | metrics | check | slice | inspect | semantic | communities | cpg | install |
|------------|:------------:|:--------:|:---:|:-------:|:-----:|:-----:|:-------:|:--------:|:-----------:|:---:|:-------:|
| `schema_version` | ✅ v2 | ✅ v2 | ✅ v1 | ✅ v1 | ✅ v1 | ✅ v1 | ✅ v1 | ✅ v2/v3 | ✅ v1 | ✅ v1 | ✅ v1 |
| Typed `*_output.rs` / analysis types | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Explicit empty arrays | ✅ | — | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Omitted optional keys | — | — | — | ✅ | ✅ | ✅ | ✅ | ✅ | — | ✅ | — |
| Composable graph topology | ✅ | — | — | — | — | ✅ | ✅ | — | — | — | — |
| Stable node UUIDs (no nil) | ✅ | — | — | — | — | — | — | — | — | — | — |

**Tests:** See [cli-io-sanity-qe.md](cli-io-sanity-qe.md) for the full coverage matrix, harness design, and extension guide.

| Layer | Cargo target | Path | Covers |
|-------|--------------|------|--------|
| 1 — Unit schema | `cli_output` | `tests/cli_output/*.rs` | Typed `*_output.rs` fixtures, serde shapes |
| 2 — Golden path | `subprocess_golden_path` | `tests/cli_output/subprocess_golden_path.rs` | Discover + blast-radius pipelines, exit 1 |
| 3 — Full sanity | `all_commands_sanity` | `tests/cli_output/all_commands_sanity.rs` | All JSON commands, sandbox `-d`, platform rules |
| Fixture | — | `tests/fixtures/tiny_polyglot_repo/` | Java + Rust polyglot subprocess input |

```bash
cargo test --test cli_output --test subprocess_golden_path --test all_commands_sanity
```

**Source:** `src/cli/blast_radius_output.rs` — `BLAST_RADIUS_SCHEMA_VERSION` is **2**.

---

## 1. `blast-radius` — schema v2

**Command:**

```bash
rgctl -f json blast-radius <SYMBOL> [--depth N] [--policy-file PATH] [--with-slices] [--class CLASS] [--file PATH]
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--depth N` | Cap `topology.impact_zone` to upstream callers within **N incoming call hops** (hop 1 = direct callers). Omits `metrics.caller_depth_limit` when unset (full closure). Score is recomputed when capped. |
| `--policy-file` | Run policy guardrails on the (possibly depth-filtered) impact zone |
| `--with-slices` | Populate `gatekeeping.handoffs` (requires full graph path) |
| `--class` / `--file` | Disambiguate overloads |

**Optional warm path:** foreground `rgctl serve` is `POST /api/query` on `127.0.0.1:8080`. Daemon HTTP uses `/{reponame}/api/query`. See [http-api.md](http-api.md).

**Source:** `src/cli/blast_radius_output.rs`  
**Cache enrichment:** `crates/rgctl-analysis/src/macro_call_index.rs`, `macro_call_lookup.rs`

### Top-level

```json
{
  "schema_version": 2,
  "target": { },
  "metrics": { },
  "topology": { },
  "gatekeeping": { }
}
```

### `target` — identification metadata (v2)

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID string | Resolved graph node id |
| `symbol` | string | Bare function/method name |
| `class_context` | string \| null | Containing class or namespace when known |
| `file_path` | string | Project-relative source path (empty if unknown) |
| `language` | string | `"java"`, `"rust"`, `"python"`, or `"unknown"` |
| `signature` | string \| omitted | Method signature when known (overload disambiguation) |
| `canonical_fqn` | string | Uniform `Class::method` (e.g. `OrderService::process`) |

- `language` comes from graph `properties.language` (set by language plugins at extract time) or file-extension fallback.
- `signature` comes from graph `Node.signature` (tree-sitter during discover).
- `canonical_fqn` normalizes Java dot notation to double-colon form; Rust `module::fn` passes through.

### `metrics` — quantitative impact

| Field | Type | Description |
|-------|------|-------------|
| `score` | number | Impact score 0–100 |
| `direct_callers_count` | integer | Immediate caller count |
| `impact_zone_size` | integer | Transitive caller count (functions only); reflects `--depth` cap when set |
| `caller_depth_limit` | integer \| omitted | Present when `--depth N` was passed; echoes the hop cap applied to `impact_zone` |

### `topology` — graph layout

| Field | Type | Description |
|-------|------|-------------|
| `scc_component_id` | integer \| null | SCC index from engine; `null` on macro-index blast lookup cache hit |
| `direct_callers` | `SymbolContext[]` | Immediate callers |
| `impact_zone` | `SymbolContext[]` | Transitive upstream callers (filtered by `--depth` when set) |

**`SymbolContext`:**

```json
{
  "id": "UUID",
  "fqn": "string",
  "file_path": "string"
}
```

- `fqn` is language-native display text from graph `qualified_name` or bare `name`.
- **Route on `target.canonical_fqn` + UUIDs**, not parsed `topology.fqn`.
- **Nil UUID policy:** entries without a resolvable graph UUID are **omitted** from `topology` (never `00000000-…`).
- After `discover`, the blast lookup cache (`macro_call_index.db` / `.bin`) stores `direct_caller_ids`, `impact_zone_ids`, and target metadata for composable chaining.

### `gatekeeping` — policy and slice tracing

| Field | Type | Description |
|-------|------|-------------|
| `policy_status` | string | `"SKIPPED"` (default), `"PASS"`, or `"VIOLATED"` |
| `violations` | array | Structured policy violations (always present; `[]` when none) |
| `handoffs` | array | Slice seeds (always present; `[]` without `--with-slices`) |

**`SliceHandoff`:** `{ "callee": string, "param": string, "index": number }`

**`PolicyViolation`** (internally tagged; discriminant is `kind`):

| `kind` | Fields |
|--------|--------|
| `domain_isolation` | `source_domain`, `reached_domain`, `node` |
| `scale_failure` | `count`, `max` |
| `cascade_hazard` | `node`, `betweenness`, `threshold` |
| `sanitization_bypass` | `sink_line`, `path_trace`, `sanitizer_node` |

### Schema history (migration)

**Legacy flat JSON (removed)** — do not parse:

```json
{
  "symbol": "CartService",
  "score": 42.3,
  "direct_callers": ["authenticate"],
  "impact_zone": ["authenticate", "main"],
  "handoffs": []
}
```

**v1 (nested)** — replaced flat root keys with `target` / `metrics` / `topology` / `gatekeeping`. `schema_version: 1`.

**v2 (current)** — adds `target.language`, `target.signature`, `target.canonical_fqn`, and optional `metrics.caller_depth_limit` when `--depth N` is set. `schema_version: 2`.

| jq path (v2) | Replaces legacy |
|--------------|-----------------|
| `.metrics.score` | `.score` |
| `.topology.direct_callers[].fqn` | `.direct_callers[]` (bare names) |
| `.topology.impact_zone[].fqn` | `.impact_zone[]` |
| `.target.id` | (new) |
| `.target.canonical_fqn` | (new — prefer for routing) |
| `.gatekeeping.handoffs` | `.handoffs` (always present in v1+) |

**FQN policy:** route on `target.canonical_fqn` (`Class::method`) and `topology.*.id` (UUID). Treat `topology.*.fqn` as language-native display text only.

**Cache:** target metadata is written at `discover` into `macro_call_index.db` / `.bin`. Re-run `discover` after upgrading rgctl to populate v2 fields on cache hits.

### Example (Java, cache path)

```json
{
  "schema_version": 2,
  "target": {
    "id": "424d403b-1b2c-4a3d-8e9f-0c1b2a3f4e5d",
    "symbol": "process",
    "class_context": "OrderService",
    "file_path": "java/com/example/OrderService.java",
    "language": "java",
    "signature": "public void process(String orderId) {",
    "canonical_fqn": "OrderService::process"
  },
  "metrics": {
    "score": 25.05,
    "direct_callers_count": 1,
    "impact_zone_size": 3
  },
  "topology": {
    "scc_component_id": null,
    "direct_callers": [
      {
        "id": "8b2c4a3d-0c1b-4e5d-8e9f-424d403b1b2c",
        "fqn": "com.example.OrderController.checkout",
        "file_path": "java/com/example/OrderController.java"
      }
    ],
    "impact_zone": []
  },
  "gatekeeping": {
    "policy_status": "SKIPPED",
    "violations": [],
    "handoffs": []
  }
}
```

### Exit codes

- `0` — success
- `1` — `policy_status == "VIOLATED"` when `--policy-file` is set (JSON still emitted to stdout first)

---

## 1b. `serve` — HTTP dashboard + background daemon

**Foreground HTTP (one repo):**

```bash
rgctl serve -r REPO [--open]
```

Binds `http://127.0.0.1:8080/` — dashboard at `/`, GQL at `POST /api/query`. See [http-api.md](http-api.md).

**Background HTTP+MCP daemon:**

```bash
rgctl daemon start [--host HOST] [--port PORT]
rgctl serve -r REPO --daemon [--idle-secs SECS]
```

Default bind `0.0.0.0:8080`; catalog at `/`, per-repo routes under `/{reponame}/`, MCP at `/mcp`. Cache lives under `~/.rgctl/cache/{reponame}/` unless `--daemon-home` / storage override is set.

**Role:** Shared in-memory graph + command service for CLI routing, HTTP, and MCP. Opt out with **`--no-daemon`** (in-process, `{repo}/.rgctl/`).

**Requires:** prior `discover` (via daemon or `--no-daemon`) producing `graph.snapshot.bin`.

---

## 2. `discover` — schema v2 (stdout JSON)

**Command:**

```bash
rgctl -f json discover PATH [--languages LANGS] [--exclude PATTERNS] [--with-security] [--with-cfg] [--with-taint] [--write-json-graph]
```

**Source:** `src/cli/discover_output.rs`, `src/cli/discover_impl.rs`

With `-f json`, discover **suppresses** progress bars and human status lines on stderr (logging quiet unless `-v`). **Stdout** receives a single telemetry object after ingestion completes. Artifacts under `.rgctl/` are still written.

```json
{
  "schema_version": 2,
  "command": "discover",
  "metrics": {
    "files_discovered": 10921,
    "files_indexed": 10784,
    "files_skipped": 137,
    "nodes_generated": 231410,
    "edges_generated": 562067,
    "duration_ms": 18200
  }
}
```

| Field | Source |
|-------|--------|
| `files_discovered` | `PipelineStats.files_discovered` |
| `files_indexed` | `PipelineStats.files_processed` |
| `files_skipped` | `PipelineStats.files_failed` |
| `nodes_generated` | `PipelineStats.nodes_created` |
| `edges_generated` | `PipelineStats.edges_created` |
| `duration_ms` | Full discover wall-clock (includes analysis + persist) |

Without `-f json`, discover remains human-readable text progress (unchanged).

### Artifacts on disk

| Path | When | Format |
|------|------|--------|
| `.rgctl/graph.snapshot.bin` | Always (default canonical graph) | Binary graph snapshot |
| `.rgctl/blast_engine.snapshot.bin` | Always | Binary blast engine snapshot |
| `.rgctl/macro_call_index.db` | Always | SQLite **blast-radius lookup cache** only (+ UUID + v2 target columns) |
| `.rgctl/macro_call_index.bin` | Always | Bincode companion index (same data family as `.db`) |
| `.rgctl/analysis_results.bin` | Always | Columnar analysis tables |
| `.rgctl/dashboard/` | When export succeeds | Static dashboard bundle (`index.html`, `manifest.json`, …) |
| `.rgctl/graph.db` / `.rgctl/graph.json` | `--write-json-graph` only | Legacy full graph JSON |
| `.rgctl/analysis/cfg_pdg.archive.bin` | `--with-cfg` or `--with-taint` | CFG + PDG for `--with-slices` |
| `.rgctl/analysis/*.json` | `--with-cfg` or `--with-taint` | Per-function analysis storage (taint, CFG, PDG) |
| `.rgctl/dashboard/taint_index.json` | `--with-cfg` or `--with-taint` | Dashboard taint catalog (see [json-api.md](json-api.md) §12) |

---

## 3. `gql` — schema v1

**Command:**

```bash
rgctl -f json gql "<QUERY>" [--explain] [--macro NAME]
```

**Source:** `src/cli/gql_output.rs`

```json
{
  "schema_version": 1,
  "rows": [
    [
      {
        "binding": "string",
        "node": "string",
        "type": "string",
        "qualified_name": "string (optional)",
        "file": "string | null",
        "community_id": "number (optional)",
        "label": "string (optional)",
        "member_count": "number (optional)",
        "properties": "object (optional, allowlisted keys)"
      }
    ]
  ],
  "count": 0,
  "explain": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `rows` | array | One element per result row; each row is an array of bindings |
| `count` | integer | Always equals `rows.length` |
| `explain` | boolean | Mirrors `--explain` flag |
| `binding` | string | Variable name from the `MATCH` pattern |
| `node` | string | Matched node bare name (or community label) |
| `type` | string | `NodeType` debug name, or `"Community"` for virtual overlay nodes |
| `qualified_name` | string \| omitted | Graph FQN when present; filter with `WHERE n.qualified_name = '...'` (not `n.name`) |
| `file` | string \| null | Source path when present on the node |
| `community_id` | number \| omitted | Community id on `:Community` rows |
| `label` | string \| omitted | Heuristic community label |
| `member_count` | number \| omitted | Community size |
| `properties` | object \| omitted | Allowlisted extract properties (`is_lambda`, `throws`, …) |

**Note:** The explain **plan** is not included in JSON; it prints to text mode only. Virtual `:Community` / `community_id` require `.rgctl/analysis_results.bin` after `discover`.

---

## 4. `metrics` — schema v1

**Command:**

```bash
rgctl -f json metrics [--pagerank] [--betweenness] [--communities] [--iterations N]
```

**Source:** `src/cli/metrics_output.rs`, `src/cli/metrics.rs`

Default (no section flags) computes **all three** sections.

```json
{
  "schema_version": 1,
  "pagerank": {
    "top": [
      { "node": "UUID string", "pagerank": 0.0 }
    ],
    "converged": true,
    "iterations": 20,
    "max_delta": 0.0
  },
  "betweenness": [
    { "node": "UUID string", "score": 0.0 }
  ],
  "communities": {
    "count": 0,
    "modularity": 0.0,
    "assignments": 0
  }
}
```

| Section | When present | Notes |
|---------|--------------|-------|
| `pagerank` | `--pagerank` or default (all) | `top` capped at 20 nodes |
| `betweenness` | `--betweenness` or default | Top-level **array**, top 20 |
| `communities` | `--communities` or default | `assignments` = number of labeled nodes |

Omitted keys: sections not requested are **absent** (not `null`, not `[]`). Serialization uses `Option` + `#[serde(skip_serializing_if = "Option::is_none")]` via `MetricsJsonResponse`.

---

## 5. `check` — schema v1

**Command:**

```bash
rgctl -f json check --policy-file PATH
```

**Source:** `src/cli/check_output.rs`

```json
{
  "schema_version": 1,
  "policy": "path/to/policy.json",
  "violations": [
    {
      "symbol": "string",
      "error": "string",
      "violation": "string"
    }
  ],
  "passed": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `policy` | string | Path passed to `--policy-file` |
| `violations` | array | Always present; empty when passing |
| `passed` | boolean | `true` iff `violations` is empty |

**Violation entry** (one of `error` or `violation`; the other is omitted):

```json
{ "symbol": "foo", "error": "engine or policy error text" }
```

```json
{ "symbol": "foo", "violation": "cascade hazard: node … betweenness …" }
```

### Exit codes

- `0` — `passed == true`
- `1` — `passed == false`

---

## 6. `slice` — schema v1

**Command:**

```bash
rgctl -f json slice FILE --line N --variable VAR [--view cfg|pdg|text] [--direction backward|forward] [--taint]
```

**Source:** `src/cli/slice_output.rs`

### CFG view (`--view cfg`)

```json
{
  "schema_version": 1,
  "file": "string",
  "function": "string",
  "view": "cfg",
  "nodes": [
    {
      "id": "block_0",
      "block_index": 0,
      "start_line": 1,
      "end_line": 5,
      "statements": [
        { "line": 1, "kind": "Expression", "text": "let x = 1;" }
      ]
    }
  ],
  "edges": [
    { "source": "block_0", "target": "block_1", "kind": "next" }
  ]
}
```

### PDG view (`--view pdg`)

```json
{
  "schema_version": 1,
  "file": "string",
  "function": "string",
  "view": "pdg",
  "nodes": [
    { "id": "node_0", "line": 42, "label": "let tmp = ctx;", "kind": "Expression" }
  ],
  "edges": [
    { "source": "node_1", "target": "node_0", "kind": "data", "variable": "ctx" }
  ]
}
```

### Text slice view (default `--view text`)

Includes line list **and** PDG subgraph topology for the slice:

```json
{
  "schema_version": 1,
  "file": "string",
  "criterion": { "line": 42, "variable": "ctx" },
  "direction": "backward",
  "reduction_percent": 65.0,
  "lines": [40, 42],
  "nodes": [ { "id": "node_0", "line": 42, "label": "...", "kind": "..." } ],
  "edges": [ { "source": "node_1", "target": "node_0", "kind": "data", "variable": "ctx" } ]
}
```

### Taint mode (`--taint`)

```json
{
  "schema_version": 1,
  "file": "string",
  "function": "string",
  "line": 0,
  "variable": "string",
  "taint": true,
  "flows": 0,
  "vulnerable": 0
}
```

---

## 7. `inspect` — schema v1

**Command:**

```bash
rgctl -f json inspect SYMBOL --layer cfg|pdg|dom [layer options]
```

**Source:** `src/cli/inspect_output.rs`

### CFG layer

```json
{
  "schema_version": 1,
  "symbol": "string",
  "layer": "cfg",
  "pruned": false,
  "nodes": [ { "id": "block_0", "block_index": 0, "start_line": 1, "end_line": 5, "statements": [] } ],
  "edges": [ { "source": "block_0", "target": "block_1", "kind": "next" } ]
}
```

### PDG layer

```json
{
  "schema_version": 1,
  "symbol": "string",
  "layer": "pdg",
  "nodes": [
    { "id": "node_0", "line": 1, "label": "...", "kind": "...", "defined": ["x"], "used": ["y"] }
  ],
  "edges": [ { "source": "node_0", "target": "node_1", "kind": "control" } ],
  "data_deps": 0,
  "control_deps": 0
}
```

`defined` / `used` appear when `--def-use` is set.

### Dominance layer

```json
{
  "schema_version": 1,
  "symbol": "string",
  "layer": "dom",
  "nodes": [ { "block_index": 0, "start_line": 10, "end_line": 15 } ],
  "idom": [ { "block": 1, "immediate_dominator": 0 } ],
  "frontiers": [ { "block": 0, "frontier_blocks": [2, 3] } ]
}
```

Block references use stable **`block_index`** integers (sorted by `start_line`), not debug strings.

**Other formats:** `--format mermaid` and `--format graphviz` emit diagram text for CFG/dom layers (not JSON).

---

## 8. `export` — file output (not stdout JSON)

**Command:**

```bash
rgctl export --export-format json --export-output graph.json [--query "…"]
```

Writes to `-o`; stdout is a one-line summary unless output is redirected via global `-o`.

| `--format` | File content |
|------------|--------------|
| `json` | `CodeGraph::export_json()` (same family as `graph.db`) |
| `graphml` | GraphML XML |
| `graphviz` | DOT |
| `mermaid` | Mermaid flowchart |

---

## 9. `semantic`

See [json-api.md §15](json-api.md#15-semantic) for TypeScript shapes and jq recipes.

| Subcommand | `schema_version` | Source |
|------------|-----------------:|--------|
| `semantic index` | **2** | `SEMANTIC_INDEX_CLI_SCHEMA_VERSION` |
| `semantic query` | **3** | `SEMANTIC_QUERY_CLI_SCHEMA_VERSION` |
| `semantic distill` | **1** | teacher → RBVK write |

| Field (index) | Type | Notes |
|---------------|------|-------|
| `model_id` | string | Embedder / model id |
| `dimensions` | number | Default **256** |
| `functions_indexed` | number | Entries written |
| `path` | string | Index file path |
| `build_stats` | object? | Incremental counters |

| Field (query hit) | Type | Notes |
|-------------------|------|-------|
| `node_id` | string | Graph node UUID |
| `name` | string | Function name |
| `distance` | number | Hamming distance |
| `score` | number | Similarity or fused score |
| `fused_score` | number? | Present when fusion ranking applied |

---

## 10. `communities`

See [json-api.md §16](json-api.md#16-communities).

| Field | Type | Notes |
|-------|------|-------|
| `schema_version` | number | **1** |
| `modularity` | number | Newman Q |
| `written` | bool | True after `label --write` |
| `communities[].id` | number | Community id |
| `communities[].label` | string | Heuristic label |
| `communities[].member_count` | number | Members |

---

## 11. `cpg`

See [json-api.md §17](json-api.md#17-cpg). Requires `discover --with-cfg`.

| Subcommand | Primary fields |
|------------|----------------|
| `status` | `archive_present`, `function_count`, `field_write_*`, `ast_skeleton_*` |
| `function` | `id`, `name`, `has_l_proc`, `is_constructor` |
| `calls` | `edges[]` |
| `mutations` | `mutations[]` (file, line, member, function, …) |
| `flows` | `steps[]` |
| `export` | file output (`--format` / `--output`), not stdout JSON |

All `-f json` CPG payloads use `schema_version: 1`.

---

## 12. `install`

See [json-api.md §18](json-api.md#18-install). Source: `src/cli/install_output.rs`. Does not require `discover`.

| Field | Type | Notes |
|-------|------|-------|
| `schema_version` | number | **1** |
| `command` | string | Always `"install"` |
| `skill` | string | Always `"rgctl"` |
| `repo` | string | Absolute repository root |
| `force` | bool | Whether `--force` was set |
| `writes[].host` | string | `"claude"` or `"cursor"` |
| `writes[].path` | string | Absolute dest path |
| `writes[].status` | string | `created` / `unchanged` / `overwritten` / `skipped_exists` |

---

## Verification

```bash
# Typed schema sanity (unit fixtures per command)
cargo test --test cli_output

# Subprocess golden path (discover + blast-radius)
cargo test --test subprocess_golden_path

# Full platform I/O audit (all structured commands, sandbox -d)
cargo test --test all_commands_sanity

# Combined CI gate
cargo test --test cli_output --test subprocess_golden_path --test all_commands_sanity
```

---

## Remaining gaps

- **HTML dashboard:** still uses discover-time node properties, not CLI JSON shapes
- **Rust plugin:** does not set `properties.language` on graph nodes yet (v2 falls back to `.rs` extension)
- **Re-run `discover`** on repos indexed before P2 to populate blast lookup cache UUID + v2 target columns
