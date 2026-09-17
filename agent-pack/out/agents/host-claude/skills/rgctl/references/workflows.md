# Workflow Scenarios

Worked NL scenarios showing the discover → query → reason → act pattern for common tasks.

## Table of Contents

- [Migration & Audit](#migration--audit)
- [Konveyor / Kantra Migration Rules](#konveyor--kantra-migration-rules)
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

## Konveyor / Kantra Migration Rules

Native evaluation of [Konveyor Kantra](https://github.com/konveyor/kantra) rules against the rgctl graph and source cache. Release builds embed Konveyor `stable/java` (~2.6k rules); no external Kantra CLI required.

### 5a. Default Kantra discover

**User intent:** *"Run Konveyor migration rules on this Java codebase"*

```bash
rgctl discover . -l java --with-kantra
# violations: .rgctl/kantra_findings.json
# rules in graph: KantraRule / KantraRuleset nodes (GQL)
```

Report `catalog_id`, `evaluated_rules`, violation count, sample hits (`rule_id`, `file`, `line`, `matched_by`), and top `skipped_rules` reasons.

### 5b. Target-filtered eval

**User intent:** *"What Quarkus migration rules apply?" / "Audit for Spring Boot 3+"*

```bash
rgctl discover . -l java --with-kantra --kantra-target quarkus
# or: --kantra-target spring-boot3+
```

`target_filter` appears in `kantra_findings.json`. Only rules with `konveyor.io/target=<NAME>` labels are evaluated.

### 5c. Rules inventory (GQL)

**User intent:** *"List migration rules indexed in the graph" / "Which rules target Quarkus?"*

```bash
rgctl -f json gql "MATCH (r:KantraRule) RETURN r LIMIT 20"
# Konveyor labels are node properties — use backtick-quoted keys:
rgctl -f json gql 'MATCH (r:KantraRule) WHERE r.`konveyor.io/target` = '\''quarkus'\'' RETURN r'
```

`KantraRuleset` nodes link to rules via `CONTAINS` edges. After full eval, `VIOLATES` edges connect rules to code nodes; `kantra_findings.json` has line-level detail and enrichment.

### 5d. Fixture / CI override

**User intent:** *"Run a small custom ruleset in CI"*

```bash
rgctl discover . --with-kantra --kantra-rules tests/fixtures/kantra-rules
```

Mutually exclusive with `--kantra-catalog`. Embedded catalog is the default when neither override is set.

### 5e. Index only

**User intent:** *"Index rules into the graph without running eval"*

```bash
rgctl discover . --with-kantra --kantra-index-only
```

Useful when you only need GQL rule inventory. Eval stage is skipped; `kantra_findings.json` is not written.

**Pitfalls:**
- Does **not** require `--with-cfg`
- Many upstream Konveyor rules use unsupported providers (`builtin.xml`, `java.dependency`) or Windup-style regex — expect a large `skipped_rules` list with full catalog
- Re-run discover after rule/catalog changes; kantra index rewrites `graph.snapshot.bin` at end of pipeline

**See:** [User guide — Kantra](../../docs/user-guide.md#kantra-migration-rules---with-kantra), [JSON API](../../docs/json-api.md#kantra_findingsjson), [KANTRA_ARCHITECTURE_OPTIONS.md](../../KANTRA_ARCHITECTURE_OPTIONS.md)

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

See [docs/http-api.md](../../docs/http-api.md). For IDE agents spawn `rgctl -f json` subprocesses; optional `rgctl serve` for repeated HTTP queries on one repo.

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
| `--with-kantra` | Konveyor Kantra eval + `KantraRule` graph index (embedded catalog) |
| `--kantra-target <name>` | Eval only `konveyor.io/target=<name>` rules |
| `--kantra-rules <dir>` | Override embedded catalog with one ruleset directory |
| `--kantra-catalog <root>` | Override with local rulesets tree |
| `--kantra-index-only` | Index rules into graph; skip eval |

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
