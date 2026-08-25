# Workflow Scenarios

Worked NL scenarios showing the discover → query → reason → act pattern for common tasks.

## Table of Contents

- [Migration & Audit](#migration--audit)
- [Intent Discovery & Subsystem Mapping](#intent-discovery--subsystem-mapping)
- [Pre-Refactor Safety Analysis](#pre-refactor-safety-analysis)
- [CI Gates & Policy](#ci-gates--policy)
- [Advanced Patterns](#advanced-patterns)

---

## Migration & Audit

### 1. Migration Plan

**User intent:** *"Generate a complete migration plan for this codebase"*

```bash
rgctl discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints \
  --migration-preset hybrid_default --migration-order scheduled
# read .rgctl/migration_plan.json (and/or dashboard Migration tab via serve --open)
```

Discover stdout (`-f json`) is **telemetry** — not the plan body. Report path + preset/order used + top `packages[]` by priority/step.

**Migration presets:**
- `hybrid_default` - Balanced approach (default)
- `foundational_first` - Migrate core/base libraries first
- `dense_cluster` - Tackle tightly-coupled modules together
- `risk_mitigation` - Minimize blast radius per step

**Migration orders:**
- `scheduled` - Dependency-aware sequence (default)
- `priority` - Highest-impact packages first

### 2. Hotspots

**User intent:** *"Which core functions are bottlenecks / central dependencies?"*

```bash
rgctl -f json metrics --pagerank
```

Report `.pagerank.top` nodes + why they are risky to change. Resolve UUIDs to function names using `cpg function`.

### 3. Function Inventory

**User intent:** *"Give me an inventory of functions … candidates to delete or shrink"*

```bash
rgctl -f json gql --macro-name all_functions unused
```

`all_functions` → full inventory (`count` + `rows`). `unused` is a **placeholder**. Cross-check with blast-radius / CALL queries before deletes.

### 4. Named Communities

**User intent:** *"What architectural communities / packages does the graph see?"*

```bash
rgctl -f json gql --macro-name all_communities unused
# prefer for labels + modularity: rgctl -f json communities list
```

Lists communities — **not** "orphaned modules." Inspect members and call edges before proposing a prune.

### 5. CPG Export

**User intent:** *"Export a GraphSON archive to preserve the baseline before refactoring"*

```bash
rgctl cpg export --format graphson --output cpg.json --path-contains src/
```

Writes a **file**; success is typically a text summary. Needs prior `discover --with-cfg` for a useful L_proc-rich export.

---

## Intent Discovery & Subsystem Mapping

### 6. NL Function Search

**User intent:** *"Where is the code that handles our checkout flow?"*

```bash
rgctl semantic index                    # opt-in; default vocab. extras: --embedder code-daemon|hash
rgctl -f json semantic query "checkout flow" --limit 10
```

Report top `hits[]` (`name`, `score`, `file_path`).

### 7. Community Semantic

**User intent:** *"Which architectural subsystem owns checkout?"*

```bash
rgctl -f json semantic query "checkout" --scope community --limit 10
```

Hits are pooled **community** results (same `hits[]` contract).

### 8. Pattern Search

**User intent:** *"Find all Service classes … naming consistency"*

```bash
rgctl -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
```

Suffix-only — `*middle*` silently returns 0. For contains-style search, use `semantic query "Service"` instead.

### 9. Community Members

**User intent:** *"List all the functions inside Community 12"*

```bash
rgctl -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"
```

---

## Pre-Refactor Safety Analysis

Example: preparing to refactor `updateQuantity`

### 10. Blast Radius

**User intent:** *"What's the impact if I change the signature of `updateQuantity`?"*

```bash
rgctl -f json blast-radius updateQuantity --depth 2
```

Report `metrics.score`, `topology.direct_callers`, impact size. Add `--class` / `--file` if ambiguous.

### 11. Call Neighborhood

**User intent:** *"Show me the call stack surrounding `updateQuantity` up to 3 hops"*

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function)
  WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"
```

### 12. AST Skeleton

**User intent:** *"Inspect the AST skeleton of `updateQuantity` to check its structure"*

```bash
rgctl discover . --with-ast-skeleton
rgctl -f json cpg ast updateQuantity
```

Coarse skeleton (`kind`, lines, `label`) — **not** a typed signature API (`params` / `return_type` are not emitted).

### 13. Status + Line Slice

**User intent:** *"Confirm the CFG archive is ready, then slice how `quantity` is used in `updateQuantity`"*

```bash
rgctl -f json cpg status
rgctl -f json cpg slice src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --view pdg
```

**`cpg slice` has no `--symbol`.** For whole-function CFG/PDG, use `inspect <Symbol> cfg|pdg` or `cpg pdg <Symbol>`.

### 14. Field Mutations

**User intent:** *"Check where `ShoppingCart` object fields are mutated"*

```bash
rgctl -f json cpg mutations --type ShoppingCart --exclude-ctors
```

### 15. Data Flows

**User intent:** *"Trace how the `quantity` variable flows into database queries"*

```bash
rgctl -f json cpg flows src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --direction forward
```

### 16. Loop-Carried DFG

**User intent:** *"Check for loop-carried dependencies that prevent parallelization"*

```bash
rgctl discover . --with-cfg --with-dfg-loops
rgctl -f json inspect BatchProcessor.process pdg --edge-layer data
```

`--with-dfg-loops` **tags** edges during discover — it does not print a dedicated loop-hazard array. Look for `loop_carried` on PDG data deps.

---

## CI Gates & Policy

### 17. Policy Check

**User intent:** *"Validate changes against project policies before committing"*

```bash
rgctl -f json check --policy-file policy.json
```

Blast-radius policy schema (`max_impact_nodes`, `forbidden_crossings`, …) — see [docs/policy-format.md](../../docs/policy-format.md). Named rules like `no-controller-direct-db-access` are **not** built-in ids. Report `passed` + `violations`.

---

## Advanced Patterns

### Relationship Between Two Symbols

**User intent:** *"What's the relationship between A and B?"*

1. Resolve symbols → bounded CALLS/DEPENDSON traversal
2. Report hops, shared neighbors, files
3. If no direct path but asymmetric dependency, fall back to `blast-radius` on each

### Concept Search with 0 LIKE Hits

If GQL LIKE returns 0 for a concept (e.g., "ingress", "gateway"):
1. Try `communities list` and grep labels
2. Try `semantic query "<concept>"`
3. Broaden LIKE to non-Function node types (Modules, Classes)

Concepts often live in package/directory paths or type names, not bare function names.

### HTTP Session for Many Queries

**User intent:** *"I need to run many queries interactively"*

```bash
rgctl -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query
# {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](../../docs/http-api.md). Prefer this over `serve --daemon`.

---

## Migration Feature-Flag Cheat Sheet

| Flag | Enables |
|------|---------|
| `--with-cfg` | CFG/PDG/dominance archive (slice, inspect, cpg PDG) |
| `--with-taint` | Discover-time taint (implies CFG as needed) |
| `--with-security` | Secret scanning |
| `--with-dashboard` | `.rgctl/dashboard/` bundle |
| `--with-harmonic` | Harmonic centrality (migration ranking; expensive) |
| `--export-migration-hints` | Write `migration_plan.json` |
| `--with-ast-skeleton` | AST skeleton for `cpg ast` |
| `--with-dfg-loops` | Tag loop-carried data deps on PDG |
| `--migration-preset <name>` | Strategy: `hybrid_default`, `foundational_first`, `dense_cluster`, `risk_mitigation` |
| `--migration-order <name>` | Roadmap sort: `scheduled` (dependency-aware), `priority` (score rank) |

Migration-oriented discover (heavy):

```bash
rgctl discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints \
  --migration-preset foundational_first --migration-order scheduled
# then read .rgctl/migration_plan.json (or dashboard copy)
```

Choose `--migration-preset` to match user intent. Use `--migration-order priority` when the user wants highest-impact packages first instead of a dependency-safe sequence.

---

## See Also

- [Command Encyclopedia](command-encyclopedia.md) - Detailed command reference
- [Migration Planning Guide](../../docs/guides/migration-planning.md) - In-depth migration strategies
- [Agent Recipes](../../docs/agent-recipes.md) - Copy-paste recipes
