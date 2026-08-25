# AI Agent Demo Script: `rgBuilder` Agent Skill in Action

This script demonstrates how **`rgBuilder`** operates when installed as a native **Agent Skill** in environments like Gemini Agent, OpenCode, or Claude Agent.

When registered as a skill, the agent does not start with terminal commands—it starts with a **user prompt in natural language**. The LLM decides when and how to invoke the `rgBuilder` tool to fetch graph-backed code facts before reasoning or applying refactoring edits.

**Accuracy notes (verified against CLI):**

- Commands and flags below match `rgctl --help` / subcommand help (as of this repo).
- Sample JSON is **illustrative but schema-aligned** (`schema_version`, real field names). It is not a live capture from one fixture run.
- Prefer the in-tree **ecommerce-java** fixture for demos (`rgbuilder-tests/ecommerce-java`). Symbol names in scenarios are illustrative.
- Dashboard / migration JSON are **opt-in** (`--with-dashboard`, `--export-migration-hints`).
- GQL `--macro-name … unused`: the trailing `unused` (or `x`) is only a **required QUERY placeholder** when using a macro — it does **not** mean “find unused code.”

---

## Agent Loop Pattern

```
 ┌───────────────────────┐
 │ 1. USER PROMPT        │  "How do I safely refactor the checkout flow?"
 └──────────┬────────────┘
            │
            ▼
 ┌───────────────────────┐
 │ 2. AGENT TOOL CALL    │  tool_use: rgctl(command="-f json semantic query 'checkout flow'")
 └──────────┬────────────┘
            │
            ▼
 ┌───────────────────────┐
 │ 3. GRAPH FACTS (JSON) │  {"schema_version":3,"hits":[{"name":"priceShoppingCart","score":0.81…}]}
 └──────────┬────────────┘
            │
            ▼
 ┌───────────────────────┐
 │ 4. LLM REASONING      │  Identifies callers, mutation risks, and design pattern edits
 └──────────┬────────────┘
            │
            ▼
 ┌───────────────────────┐
 │ 5. CODE EDIT / ACTION │  Applies refactored code & verifies with rgctl(check)
 └───────────────────────┘

```

**Prerequisite (once per repo):** `discover` (and `semantic index` when using Search). Deep analysis (`cpg`, `inspect`, slice/taint) needs `discover --with-cfg` (and related flags).

---

# Flow 1: High-Level Modernization & System Audit

*In this flow, a developer asks the agent for a global modernization audit of a legacy codebase.*

---

### Scenario 1: Migration Plan Generation

* **User Prompt:** *"Generate a complete migration plan to help us modernize this repository."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints"
}
```

* **What actually happens:** Discover writes artifacts under `.rgbuilder/` — notably `.rgbuilder/migration_plan.json` (from `--export-migration-hints`) and, with `--with-dashboard`, `.rgbuilder/dashboard/migration_graph.json` + dashboard `migration_plan.json`. With `-f json`, stdout is **discover telemetry** (`schema_version`, `metrics`, …), not the plan body itself. The agent should **read the plan file** (or open the Migration tab via `serve --open`).

* **rgBuilder Skill Output (illustrative — plan file excerpt):**
```json
{
  "schema_version": 2,
  "preset": "hybrid_default",
  "order": "scheduled",
  "packages": [
    {"name": "com.example.ecommerce.payment", "priority": 0.91, "step": 1},
    {"name": "com.example.ecommerce.cart", "priority": 0.72, "step": 2}
  ]
}
```

* **LLM Reasoning & Action:**
> "rgBuilder wrote a migration roadmap under `.rgbuilder/migration_plan.json` (and the dashboard Migration bundle). I will open the highest-priority packages first and use blast-radius before editing shared symbols."

---

### Scenario 2: Architectural Hotspots

* **User Prompt:** *"Which core functions are major system bottlenecks or central dependencies?"*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json metrics --pagerank"
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "pagerank": {
    "top": [
      {"node": "execute", "pagerank": 0.142},
      {"node": "getConnection", "pagerank": 0.098}
    ],
    "converged": true,
    "iterations": 20,
    "max_delta": 1.2e-7
  }
}
```

* **LLM Reasoning & Action:**
> "`execute` ranks highest on PageRank. Modifying its interface carries high risk — I will check blast-radius and wrap or adapt callers before changing the signature."

---

### Scenario 3: Inventory Functions

* **User Prompt:** *"Give me an inventory of functions in this repo so we can spot candidates to delete or shrink."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json gql --macro-name all_functions unused"
}
```

> **Note:** `all_functions` expands to `MATCH (f:Function) RETURN f` — a full inventory. The word `unused` is a **placeholder argument**, not a dead-code filter. Finding zero-caller functions needs a follow-up query or offline analysis of CALL edges (there is no built-in “unused functions” macro).

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "count": 292,
  "rows": [
    [
      {
        "binding": "f",
        "node": "formatLegacyCsvExport",
        "type": "Function",
        "file": "src/utils/exporter.ts"
      }
    ]
  ]
}
```

* **LLM Reasoning & Action:**
> "I have a function inventory (`count` + `rows`). Next I will cross-check suspicious names with blast-radius / CALL queries before deleting anything."

---

### Scenario 4: List Named Communities

* **User Prompt:** *"What architectural communities / packages does the graph see?"*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json gql --macro-name all_communities unused"
}
```

> **Note:** `all_communities` lists label-propagation communities (virtual `:Community`). It does **not** mean “orphaned modules.” For labeled list + modularity, prefer `communities list` / `-f json communities list`.

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "count": 40,
  "rows": [
    [
      {
        "binding": "c",
        "type": "Community",
        "community_id": 19,
        "label": "legacy_v1_reports",
        "member_count": 14
      }
    ]
  ]
}
```

* **LLM Reasoning & Action:**
> "Community `19` (`legacy_v1_reports`, 14 members) is a candidate subsystem. I will inspect members and call edges into/out of it before proposing a prune."

---

### Scenario 5: Export Whole-Graph CPG

* **User Prompt:** *"Export our graph structure to GraphSON so we can archive the baseline before refactoring."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "cpg export --format graphson --output cpg.json --path-contains src/"
}
```

> Requires prior `discover --with-cfg` for a useful L_proc-rich export. Writes a **file** (`--output`); success is typically a text summary, not a rich stdout JSON document.

* **rgBuilder Skill Output (illustrative summary):**
```text
Wrote hybrid CPG export to cpg.json (graphson)
```

* **LLM Reasoning & Action:**
> "Baseline CPG exported to `cpg.json`. We can diff or re-import later to verify structural drift after the refactor."

---

# Flow 2: Intent-Driven Feature Discovery & Subsystem Mapping

*In this flow, the developer asks broad domain questions. The agent uses vector semantic search and graph querying skills to navigate the codebase.*

---

### Scenario 6: Natural Language Function Search

* **User Prompt:** *"Where is the code that handles our checkout flow?"*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "semantic index"
}
tool_use: rgctl {
  "command": "-f json semantic query \"checkout flow\" --limit 10"
}
```

> `semantic index` is **opt-in** (not part of `discover`). Default embedder is **vocab** (compiled token table). Use `--embedder code-daemon` for the ONNX retriever (Git LFS weights) or `--embedder hash` in CI.

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 3,
  "query": "checkout flow",
  "model_id": "vocab-accumulate-v1",
  "dimensions": 256,
  "hits": [
    {
      "node_id": "…",
      "name": "processCart",
      "file_path": "src/checkout/service.ts",
      "distance": 42,
      "score": 0.92
    },
    {
      "node_id": "…",
      "name": "chargeToken",
      "file_path": "src/payment/gateway.ts",
      "distance": 58,
      "score": 0.87,
      "fused_score": 0.81,
      "ranking": "fusion"
    }
  ]
}
```

* **LLM Reasoning & Action:**
> "Semantic search ranked `processCart` and `chargeToken` highest. Those are the primary entry points to inspect next."

---

### Scenario 7: Community Semantic Search

* **User Prompt:** *"Which architectural subsystem owns the checkout capabilities?"*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json semantic query \"checkout\" --scope community --limit 10"
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 3,
  "query": "checkout",
  "model_id": "vocab-accumulate-v1",
  "dimensions": 256,
  "hits": [
    {
      "node_id": "12",
      "name": "Order Management & Checkout",
      "score": 0.88,
      "distance": 30
    }
  ]
}
```

> With `--scope community`, hits are **pooled community** results (needs discover analysis + semantic index). Exact hit fields follow the same `hits[]` contract as function scope.

* **LLM Reasoning & Action:**
> "Checkout maps to community `12`. I will list its members next and treat that set as the refactor boundary."

---

### Scenario 8: Find Symbol by Pattern

* **User Prompt:** *"Find all Service classes in our codebase to check their naming consistency."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json gql \"MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 20\""
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "count": 20,
  "rows": [
    [
      {
        "binding": "n",
        "node": "addItem",
        "type": "Function",
        "file": "src/cart/CartService.ts"
      }
    ]
  ]
}
```

* **LLM Reasoning & Action:**
> "Matched Service-related functions (names often bare method names in the graph). I will sample a few for naming consistency before proposing renames."

---

### Scenario 9: Community Members

* **User Prompt:** *"List all the functions inside Community 12 so I can see what's in the checkout subsystem."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json gql \"MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20\""
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "count": 3,
  "rows": [
    [{"binding": "f", "node": "processCart", "type": "Function"}],
    [{"binding": "f", "node": "validateInventory", "type": "Function"}],
    [{"binding": "f", "node": "applyTax", "type": "Function"}]
  ]
}
```

* **LLM Reasoning & Action:**
> "Community `12` includes validation, inventory, and tax helpers around checkout. I will draft a pipeline boundary around these members."

---

# Flow 3: Deep Program Analysis & Pre-Refactor Safety Slicing

*In this flow, the user asks to modify a specific symbol (`updateQuantity`). The agent executes target queries to safely plan the code edits.*

---

### Scenario 10: Impact Before Editing (Blast Radius)

* **User Prompt:** *"What's the impact if I change the signature of `updateQuantity`?"*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json blast-radius \"updateQuantity\" --depth 2"
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 2,
  "target": {
    "symbol": "updateQuantity",
    "id": "…"
  },
  "metrics": {
    "score": 12.5,
    "direct_callers_count": 2,
    "impact_zone_size": 5,
    "caller_depth_limit": 2
  },
  "topology": {
    "direct_callers": [
      {"name": "putItem", "id": "…"},
      {"name": "run", "id": "…"}
    ],
    "impact_zone": []
  }
}
```

* **LLM Reasoning & Action:**
> "Two direct callers appear in `topology.direct_callers`. I must update those call sites in the same turn before changing the signature."

---

### Scenario 11: Call Neighborhood

* **User Prompt:** *"Show me the call stack surrounding `updateQuantity` up to 3 hops out."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json gql \"MATCH (a:Function)-[:CALLS*1..3]->(b:Function) WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50\""
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "count": 2,
  "rows": [
    [
      {"binding": "a", "node": "updateQuantity", "type": "Function"},
      {"binding": "b", "node": "checkStock", "type": "Function"}
    ],
    [
      {"binding": "a", "node": "checkStock", "type": "Function"},
      {"binding": "b", "node": "get", "type": "Function"}
    ]
  ]
}
```

* **LLM Reasoning & Action:**
> "`updateQuantity` reaches `checkStock` within three CALL hops. I will inspect that path before introducing caching."

---

### Scenario 12: AST Skeleton Analysis

* **User Prompt:** *"Inspect the AST skeleton of `updateQuantity` to check its structure."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "discover . --with-ast-skeleton"
}
tool_use: rgctl {
  "command": "-f json cpg ast \"updateQuantity\""
}
```

> `--with-ast-skeleton` implies CFG. Output is a **coarse skeleton** (`kind`, lines, `label`) — not a full typed signature API (`params` / `return_type` are **not** emitted today).

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "records": [
    {
      "function_name": "updateQuantity",
      "file_path": "src/cart/CartService.ts",
      "nodes": [
        {"id": 0, "kind": "Function", "start_line": 40, "end_line": 80, "label": "updateQuantity"},
        {"id": 1, "parent": 0, "kind": "If", "start_line": 52, "end_line": 60, "label": "quantity <= 0"}
      ]
    }
  ]
}
```

* **LLM Reasoning & Action:**
> "The skeleton shows an early `If` guard around quantity. I will preserve that control structure when changing the return type in source."

---

### Scenario 13: Hybrid CPG Status / Line Slice

* **User Prompt:** *"Confirm the CFG archive is ready, then slice how `quantity` is used in `updateQuantity`."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json cpg status"
}
tool_use: rgctl {
  "command": "-f json cpg slice src/cart/CartService.ts --line 50 --variable quantity --function updateQuantity --view pdg"
}
```

> **`cpg slice` has no `--symbol`.** It wraps `slice` and requires `<FILE> --line --variable` (optional `--function`). For whole-function CFG/PDG dumps, use `inspect <Symbol> cfg|pdg` or `cpg pdg <Symbol>`.

* **rgBuilder Skill Output (JSON — status):**
```json
{
  "schema_version": 1,
  "archive_present": true,
  "function_count": 283,
  "field_write_index_present": true,
  "field_write_count": 113,
  "ast_skeleton_present": false,
  "ast_skeleton_count": 0
}
```

* **LLM Reasoning & Action:**
> "L_proc archive is present. The slice/PDG view for `quantity` at line 50 shows the dependence cone I must preserve while editing."

---

### Scenario 14: Field Mutations Detection

* **User Prompt:** *"Check where `ShoppingCart` object fields are mutated across the codebase."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json cpg mutations --type ShoppingCart --exclude-ctors"
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "type_name": "ShoppingCart",
  "exclude_ctors": true,
  "include_unresolved": false,
  "mutations": [
    {
      "file": "src/cart/CartService.ts",
      "line": 54,
      "member": "items",
      "function": "updateQuantity",
      "code": "this.items = …",
      "is_constructor": false,
      "kind": "…"
    }
  ]
}
```

* **LLM Reasoning & Action:**
> "Non-constructor writes to `ShoppingCart` fields are listed in `mutations[]`. I will review each site for encapsulation violations."

---

### Scenario 15: Data Flows / Slice Tracking

* **User Prompt:** *"Trace how the `quantity` variable flows from `CartService.ts` into database queries."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json cpg flows src/cart/CartService.ts --line 50 --variable quantity --function updateQuantity --direction forward"
}
```

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "steps": [
    {"line": 50, "code": "quantity = …"},
    {"line": 78, "code": "repo.updateQuantity(…, quantity)"}
  ]
}
```

* **LLM Reasoning & Action:**
> "Forward flow from line 50 reaches the repository update. I will add validation at the service boundary before that call."

---

### Scenario 16: Loop-Carried DFG Analysis

* **User Prompt:** *"Check if our batch item updates have loop-carried dependencies that prevent parallelization."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "discover . --with-cfg --with-dfg-loops"
}
tool_use: rgctl {
  "command": "-f json inspect BatchProcessor.process pdg --edge-layer data"
}
```

> `--with-dfg-loops` **tags** loop-carried `DataDependency` edges during discover; it does **not** print a dedicated loop-hazard JSON array on discover stdout. Inspect the PDG (or dashboard Dataflow) afterward for `loop_carried` tags.

* **rgBuilder Skill Output (illustrative — PDG edge property):**
```json
{
  "schema_version": 1,
  "layer": "pdg",
  "edges": [
    {
      "kind": "DataDependency",
      "loop_carried": true,
      "from": "…",
      "to": "…"
    }
  ]
}
```

* **LLM Reasoning & Action:**
> "A loop-carried data dependence is tagged on the PDG. I will not naively parallelize that loop without breaking the carried dependency."

---

# Flow 4: Automated Verification & Policy Enforcement

*In this final step, the agent edits the code and runs verification policies through the skill.*

---

### Scenario 17: CI Policy Gate Check

* **User Prompt:** *"Validate our recent code changes against project policies before committing."*
* **Agent Tool Call:**
```json
tool_use: rgctl {
  "command": "-f json check --policy-file policy.json"
}
```

> Policy files use the **blast-radius policy schema** (`max_impact_nodes`, `forbidden_crossings`, `node_domains`, …) — see [docs/policy-format.md](docs/policy-format.md). Named rules like `no-controller-direct-db-access` are **not** built-in policy ids.

* **rgBuilder Skill Output (JSON):**
```json
{
  "schema_version": 1,
  "passed": true,
  "violations": []
}
```

* **LLM Reasoning & Action:**
> "`check` passed with zero violations for the touched symbols. Safe to proceed with the commit after human review."

---

## Agent Skill Summary Matrix

| # | User Intent | Agent `rgBuilder` Skill Command | Purpose |
| --- | --- | --- | --- |
| **1** | Modernization Roadmap | `discover . … --with-dashboard --with-harmonic --export-migration-hints` | Writes migration plan + optional dashboard bundle |
| **2** | Bottleneck Detection | `-f json metrics --pagerank` | PageRank hotspots (`.pagerank.top`) |
| **3** | Function Inventory | `-f json gql --macro-name all_functions unused` | Lists functions (`unused` = placeholder) |
| **4** | Community Inventory | `-f json gql --macro-name all_communities unused` | Lists communities (not “orphans”) |
| **5** | Graph State Archiving | `cpg export --format graphson --output FILE` | Writes GraphSON/GraphML file |
| **6** | Semantic Search | `semantic index` + `-f json semantic query "<text>"` | Intent → function hits |
| **7** | Subsystem Mapping | `-f json semantic query "<text>" --scope community` | Community-scoped search |
| **8** | Pattern Matching | `-f json gql "MATCH (n:Function) WHERE…"` | Structural name patterns |
| **9** | Community Inspection | `-f json gql "MATCH (f:Function) WHERE f.community_id…"` | Members of a community |
| **10** | Impact / Blast Radius | `-f json blast-radius <Symbol> --depth N` | Call-graph impact |
| **11** | Call Neighborhood | `-f json gql "MATCH (a)-[:CALLS*1..3]->(b)…"` | Multi-hop CALL paths |
| **12** | AST Skeleton | `discover . --with-ast-skeleton` / `cpg ast <Symbol>` | Coarse AST skeleton |
| **13** | Status + Line Slice | `-f json cpg status` / `cpg slice FILE --line --variable` | Archive readiness + slice |
| **14** | State Mutation Check | `-f json cpg mutations --type <Type>` | Field-write sites |
| **15** | Variable Flow Tracking | `-f json cpg flows FILE --line --variable --function` | Forward/backward flows |
| **16** | Loop-Carried DFG | `discover . --with-cfg --with-dfg-loops` then `inspect … pdg` | Tag + inspect loop-carried deps |
| **17** | Policy Enforcement | `-f json check --policy-file policy.json` | CI blast/policy gate |

---

## See also

- [AGENTS.md](AGENTS.md) · [docs/agent-recipes.md](docs/agent-recipes.md)
- [docs/json-api.md](docs/json-api.md) · [docs/cli-output-schemas.md](docs/cli-output-schemas.md)
- [docs/policy-format.md](docs/policy-format.md) · [docs/user-guide.md](docs/user-guide.md)
