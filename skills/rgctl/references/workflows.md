# Workflow Scenarios

Worked NL scenarios showing the discover → query → reason → act pattern for common tasks.

## Table of Contents

- [Index and discover](#discover-workflow)
- [Blast radius and impact](#impact-workflow)
- [Data flow and slices](#flow-workflow)
- [Semantic and structural search](#search-workflow)
- [Migration roadmap](#migrate-workflow)
- [CI and policy gates](#gate-workflow)
- [OSV triage and deps check](#vuln-workflow)
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

Common flags: `--with-cfg` (CFG/PDG archive), `--with-ast-skeleton`, `--with-dfg-loops` (loop-carried PDG tags). Migration roadmap output is the **migrate** workflow (`--export-migration-hints` → `migration_plan.json`) — do not conflate it with a plain index.

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

Fusion is on by default for semantic query. For exact graph patterns use structured verbs (`find`, `callers`, `relations`, `inventory`) — not freeform Cypher.

### NL function search

**User intent:** *"Where is the code that handles our checkout flow?"*

Report top `hits[]` (`name`, `score`, `file_path`).

### Community semantic

**User intent:** *"Which architectural subsystem owns checkout?"*

```bash
rgctl -r "$REPO" -f json semantic query "checkout" --scope community --limit 10
```

Hits are pooled **community** results (same `hits[]` contract).

### Concept search with empty hits

If `find` / name globs return 0 for a concept (e.g., "ingress", "gateway"):

1. Try `communities list` and grep labels
2. Try `semantic query "<concept>"`
3. Broaden with `find '*Gateway*' --type class` or `inventory --by type`

Concepts often live in package/directory paths or type names, not bare function names.


---

# Migrate workflow

**Primary (and only) migration deliverable for agents:** `.rgctl/migration_plan.json` from `--export-migration-hints` (dashboard Migration tab via `serve --open` when `--with-dashboard` is set).

Use this workflow for roadmap / extraction-order / "what should we migrate first?" questions. Summarize the plan — do not dump the whole JSON into context.

### Migration plan

**User intent:** *"Generate a complete migration plan for this codebase"* / *"What should we extract first?"*

```bash
rgctl discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints \
  --migration-preset hybrid_default --migration-order scheduled
# read .rgctl/migration_plan.json (and/or dashboard Migration tab via serve --open)
```

Discover stdout (`-f json`) is **telemetry** — not the plan body. Report path + preset/order used + top `packages[]` / steps by priority.

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

Optional structural probes to explain plan steps (keep `--limit` small): `inventory --by import-prefix --limit 40`, `find --annotation …`, `blast-radius` on candidate packages.

**See:** [Migration Planning Guide](../../docs/guides/migration-planning.md)


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

# Vuln / deps / reachability workflow

**When:** OSV / CVE impact — “are we affected?”, “is it reachable?”, OpenVEX.

**Index for vuln scans (required before P4–P6):**

```bash
cd "$REPO" && rgctl discover . --with-cfg
# Prefer both when you need PDG-backed sink-first confidence:
cd "$REPO" && rgctl discover . --with-cfg --with-taint
```

- Run **`--with-cfg`** for any vulnerability scan that may reach sink-first taint, blast classify, or `vuln analyze` exploitability beyond deps-only.
- Add **`--with-taint`** when you need discover-time / PDG-backed taint confidence. Without CFG, `cfg_available=false`: deps / package / callers still work, but empty taint paths are **not** PDG proof.
- If `.rgctl/` exists from a plain `discover` (no CFG), **re-discover with `--with-cfg`** before P4–P6 — do not treat the warm index as sufficient for reachability/VEX.
- Discover-time taint remains **opt-in**; on-demand `taint --sink … --source external` still needs the CFG archive from `--with-cfg`.

**Pipeline:**

| Step | Command |
|------|---------|
| P−1 Index | `rgctl discover . --with-cfg` (+ `--with-taint` for PDG confidence) |
| P0 Normalize OSV | `rgctl -f json vuln triage --osv ./advisory.json` |
| P1 Deps match | `rgctl -r "$REPO" -f json deps check --osv ./advisory.json` (+ `--include-jars lib`) |
| P2 Package imports | `rgctl -r "$REPO" -f json find --package '<coords>' --type import` |
| P3 Facade callers | `rgctl -r "$REPO" -f json callers <Symbol> --package '<coords>' --methods readValue,…` |
| P4 Boundary blast | `rgctl -r "$REPO" -f json blast-radius <Symbol> --classify-boundary` |
| P5 Sink-first taint | `rgctl -r "$REPO" -f json taint --sink ObjectMapper.readValue --source external` |
| P6 Orchestrated VEX | `rgctl -r "$REPO" -f json vuln analyze --osv ./advisory.json --include-jars lib` |

**Verdicts:** deps `not_affected` | `affected_candidate`. Analyze `exploitability`: `not_affected` | `not_exploitable` | `exploitable` | `under_investigation`. OpenVEX statuses map accordingly; unresolved sinks alone MUST NOT force `not_affected` without caller evidence. With `cfg_available=false`, prefer wording grounded in caller/deps evidence — not “PDG confirmed no path.”

**Honesty:** OSV `versions[]` may be backport series; short Maven groups use the resolver table (no silent wrong guess). Bundled JAR / `node_modules` scans are **opt-in**. Zero imports ≠ library absent when `bundled_presence` / deps match. **Xalan dual path:** `xalan:*` resolves to Apache packages **and** JDK JAXP aliases (`javax.xml.transform`, `com.sun.org.apache.xalan.internal`) with `runtime_bundled=true` — Maven absence alone does not mean no XSLT engine.

**Multi-language:** same CLI; Maven/npm/Cargo/Go/PyPI/(NuGet/Ruby/Composer stubs) resolver; Java/Jakarta + Python web boundary catalogs; declarative taint packs (`TaintRuleSet` overlays for OSV methods — no hardcoded `detect_*`).


---

## Advanced patterns

### HTTP session for many queries

**User intent:** *"I need to run many queries interactively"*

```bash
rgctl -r "$REPO" serve --open
# Dashboard UI for exploration; agents should still prefer CLI structured verbs:
#   rgctl -f json find|callers|relations|inventory|status …
```

See [docs/guides/http-server-and-dashboard.md](../../docs/guides/http-server-and-dashboard.md). For IDE agents spawn `rgctl -f json` subprocesses; optional `rgctl serve` for a local dashboard on one repo.


---

## See also

- [Command Encyclopedia](command-encyclopedia.md) - Detailed command reference
- [Migration Planning Guide](../../docs/guides/migration-planning.md) - In-depth migration strategies
- [Agent Recipes](../../docs/agent-recipes.md) - Copy-paste recipes

