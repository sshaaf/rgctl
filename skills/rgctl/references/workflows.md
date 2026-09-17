# Workflow Scenarios

Worked NL scenarios showing the discover → query → reason → act pattern for common tasks.

## Table of Contents

- [Index and discover](#discover-workflow)
- [Blast radius and impact](#impact-workflow)
- [Data flow and slices](#flow-workflow)
- [Semantic and structural search](#search-workflow)
- [Graph query language](#gql-workflow)
- [Migration roadmap](#migrate-workflow)
- [Konveyor Kantra rules](#kantra-workflow)
- [CI and policy gates](#gate-workflow)
- [Advanced patterns](#advanced-patterns)

---

# Discover workflow

**When:** First use, rebuild after large changes, or incremental `--files` update.

| Intent | Command |
|--------|---------|
| Index repo | `cd "$REPO" && rgctl discover .` or `rgctl -r "$REPO" discover` |
| Full pipeline | `discover . --full` |
| Incremental | `discover --files path1,path2` (requires existing `.rgctl/`) |

**Fast path:** If `.rgctl/` exists and the user did not ask to rebuild, do **not** re-run discover.

Common flags: `--with-cfg` (CFG/PDG archive), `--with-ast-skeleton`, `--with-dfg-loops` (loop-carried PDG tags). Migration plan output is the **migrate** workflow; Konveyor rules are the **kantra** workflow — do not conflate them with a plain index.

Artifacts live at `{repo}/.rgctl/`. Check CFG readiness with `rgctl -f json cpg status` before slice/PDG workflows.


---

# Impact workflow

**When:** Before refactors, renames, or API changes.

### Blast radius

**User intent:** *"What's the impact if I change the signature of `updateQuantity`?"*

```bash
rgctl -r "$REPO" -f json blast-radius updateQuantity --depth 2
```

Report `metrics.score`, `topology.direct_callers`, impact size. Add `--class` / `--file` if ambiguous.

### Relationship between two symbols

**User intent:** *"What's the relationship between A and B?"*

1. Resolve symbols → bounded CALLS/DEPENDSON traversal
2. Report hops, shared neighbors, files
3. If no direct path but asymmetric dependency, fall back to `blast-radius` on each


---

# Flow workflow

**When:** Slices, PDG, taint, CPG data flows. Requires `discover --with-cfg`.

Check readiness: `rgctl -f json cpg status`.

### AST skeleton

**User intent:** *"Inspect the AST skeleton of `updateQuantity` to check its structure"*

```bash
rgctl discover . --with-ast-skeleton
rgctl -f json cpg ast updateQuantity
```

Coarse skeleton (`kind`, lines, `label`) — **not** a typed signature API (`params` / `return_type` are not emitted).

### Status + line slice

**User intent:** *"Confirm the CFG archive is ready, then slice how `quantity` is used in `updateQuantity`"*

```bash
rgctl -f json cpg status
rgctl -f json cpg slice src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --view pdg
```

**`cpg slice` has no `--symbol`.** For whole-function CFG/PDG, use `inspect <Symbol> cfg|pdg` or `cpg pdg <Symbol>`.

CLI alias: `rgctl -f json slice FILE --line N --variable V [--function F] [--direction backward|forward]`.

### Field mutations

**User intent:** *"Check where `ShoppingCart` object fields are mutated"*

```bash
rgctl -f json cpg mutations --type ShoppingCart --exclude-ctors
```

### Data flows

**User intent:** *"Trace how the `quantity` variable flows into database queries"*

```bash
rgctl -f json cpg flows src/cart/CartService.ts \
  --line 50 --variable quantity --function updateQuantity --direction forward
```

### Loop-carried DFG

**User intent:** *"Check for loop-carried dependencies that prevent parallelization"*

```bash
rgctl discover . --with-cfg --with-dfg-loops
rgctl -f json inspect BatchProcessor.process pdg --edge-layer data
```

`--with-dfg-loops` **tags** edges during discover — it does not print a dedicated loop-hazard array. Look for `loop_carried` on PDG data deps.


---

# Search workflow

**When:** Natural-language or intent-based code location (requires `semantic index`).

```bash
rgctl -r "$REPO" semantic index                    # opt-in; default vocab. extras: --embedder code-daemon|hash
rgctl -r "$REPO" -f json semantic query "checkout flow" --limit 10
```

Fusion is on by default for semantic query; use GQL for exact graph patterns.

### NL function search

**User intent:** *"Where is the code that handles our checkout flow?"*

Report top `hits[]` (`name`, `score`, `file_path`).

### Community semantic

**User intent:** *"Which architectural subsystem owns checkout?"*

```bash
rgctl -r "$REPO" -f json semantic query "checkout" --scope community --limit 10
```

Hits are pooled **community** results (same `hits[]` contract).

### Concept search with 0 LIKE hits

If GQL LIKE returns 0 for a concept (e.g., "ingress", "gateway"):

1. Try `communities list` and grep labels
2. Try `semantic query "<concept>"`
3. Broaden LIKE to non-Function node types (Modules, Classes)

Concepts often live in package/directory paths or type names, not bare function names.


---

# GQL workflow

**When:** Ad-hoc graph queries, inventories, call neighborhoods.

Use **qualified_name** / FQN for classes, not bare `n.name` when disambiguating. Always use **LIMIT** on broad patterns. Explain macros before inventing raw GQL.

### Function inventory

**User intent:** *"Give me an inventory of functions … candidates to delete or shrink"*

```bash
rgctl -f json gql --macro-name all_functions unused
```

`all_functions` → full inventory (`count` + `rows`). `unused` is a **placeholder**. Cross-check with blast-radius / CALL queries before deletes.

### Named communities

**User intent:** *"What architectural communities / packages does the graph see?"*

```bash
rgctl -f json gql --macro-name all_communities unused
# prefer for labels + modularity: rgctl -f json communities list
```

Lists communities — **not** "orphaned modules." Inspect members and call edges before proposing a prune.

### Pattern search

**User intent:** *"Find all Service classes … naming consistency"*

```bash
rgctl -f json gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n LIMIT 20"
```

Suffix-only — `*middle*` silently returns 0. For contains-style search, use `semantic query "Service"` instead.

### Community members

**User intent:** *"List all the functions inside Community 12"*

```bash
rgctl -f json gql "MATCH (f:Function) WHERE f.community_id = '12' RETURN f LIMIT 20"
```

### Call neighborhood

**User intent:** *"Show me the call stack surrounding `updateQuantity` up to 3 hops"*

```bash
rgctl -f json gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function)
  WHERE a.name = 'updateQuantity' RETURN a,b LIMIT 50"
```


---

# Migrate workflow

**Primary output:** `.rgctl/migration_plan.json` (and dashboard migration view via `serve --open`).

**This workflow is not Kantra.** Do not treat `--with-kantra` or `kantra_findings.json` as the main deliverable here.

### Migration plan

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

### Hotspots

**User intent:** *"Which core functions are bottlenecks / central dependencies?"*

```bash
rgctl -f json metrics --pagerank
```

Report `.pagerank.top` nodes + why they are risky to change. Resolve UUIDs to function names using `cpg function`.

### CPG export

**User intent:** *"Export a GraphSON archive to preserve the baseline before refactoring"*

```bash
rgctl cpg export --format graphson --output cpg.json --path-contains src/
```

Writes a **file**; success is typically a text summary. Needs prior `discover --with-cfg` for a useful L_proc-rich export.

### Migration feature-flag cheat sheet

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

For extraction ordering after violations, run the **kantra** workflow separately when Konveyor rules apply.


---

# Kantra workflow

**Primary output:** `.rgctl/kantra_findings.json` and `KantraRule` / `VIOLATES` in the graph.

**This workflow is not migration roadmap export.** Do not present `migration_plan.json` as the main Kantra deliverable.

Native evaluation of [Konveyor Kantra](https://github.com/konveyor/kantra) rules against the rgctl graph and source cache. Release builds embed Konveyor `stable/java` (~2.6k rules); no external Kantra CLI required.

### Default Kantra discover

**User intent:** *"Run Konveyor migration rules on this Java codebase"*

```bash
rgctl discover . -l java --with-kantra
# violations: .rgctl/kantra_findings.json
# rules in graph: KantraRule / KantraRuleset nodes (GQL)
```

Report `catalog_id`, `evaluated_rules`, violation count, sample hits (`rule_id`, `file`, `line`, `matched_by`), and top `skipped_rules` reasons.

### Target-filtered eval

**User intent:** *"What Quarkus migration rules apply?" / "Audit for Spring Boot 3+"*

```bash
rgctl discover . -l java --with-kantra --kantra-target quarkus
# or: --kantra-target spring-boot3+
```

`target_filter` appears in `kantra_findings.json`. Only rules with `konveyor.io/target=<NAME>` labels are evaluated.

### Rules inventory (GQL)

**User intent:** *"List migration rules indexed in the graph" / "Which rules target Quarkus?"*

```bash
rgctl -f json gql "MATCH (r:KantraRule) RETURN r LIMIT 20"
# Konveyor labels are node properties — use backtick-quoted keys:
rgctl -f json gql 'MATCH (r:KantraRule) WHERE r.`konveyor.io/target` = '\''quarkus'\'' RETURN r'
```

`KantraRuleset` nodes link to rules via `CONTAINS` edges. After full eval, `VIOLATES` edges connect rules to code nodes; `kantra_findings.json` has line-level detail and enrichment.

### Fixture / CI override

**User intent:** *"Run a small custom ruleset in CI"*

```bash
rgctl discover . --with-kantra --kantra-rules tests/fixtures/kantra-rules
```

Mutually exclusive with `--kantra-catalog`. Embedded catalog is the default when neither override is set.

### Index only

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

# Gate workflow

**When:** Policy checks and temporal PR gates.

### Policy check

**User intent:** *"Validate changes against project policies before committing"*

```bash
rgctl -r "$REPO" -f json check --policy-file policy.json
```

Blast-radius policy schema (`max_impact_nodes`, `forbidden_crossings`, …) — see [docs/policy-format.md](../../docs/policy-format.md). Named rules like `no-controller-direct-db-access` are **not** built-in ids. Report `passed` + `violations`.

### Temporal PR gate

```bash
rgctl -r "$REPO" -f json pr-check --policy-file rgctl-pr-policy.json --base-ref origin/main --head-ref HEAD --strict
rgctl -r "$REPO" -f json check --temporal --policy-file policy.json --base-ref origin/main --head-ref HEAD
```

Exit code 1 means violations. Parse JSON for violation details.


---

## Advanced patterns

### HTTP session for many queries

**User intent:** *"I need to run many queries interactively"*

```bash
rgctl -r "$REPO" serve --open
# POST http://127.0.0.1:8080/api/query
# {"query":"MATCH (n:Function) RETURN n LIMIT 5"}
```

See [docs/http-api.md](../../docs/http-api.md). For IDE agents spawn `rgctl -f json` subprocesses; optional `rgctl serve` for repeated HTTP queries on one repo.


---

## See also

- [Command Encyclopedia](command-encyclopedia.md) - Detailed command reference
- [Migration Planning Guide](../../docs/guides/migration-planning.md) - In-depth migration strategies
- [Agent Recipes](../../docs/agent-recipes.md) - Copy-paste recipes

