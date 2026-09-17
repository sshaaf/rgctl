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
