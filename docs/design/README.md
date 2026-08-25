# Feature design documents

**Audience: contributors / maintainers** — not the default agent reading path. Start at [docs/README.md](../README.md) (For contributors).

Engineering designs for rgctl capabilities. Each doc follows the [migration planner](migration-planner-design.md) pattern: goals, architecture, implementation map, CLI, testing, and optional **dashboard screenshots**.

## Index

| Feature | Design doc | Dashboard tab |
|---------|------------|---------------|
| Semantic search | [semantic-search-design.md](semantic-search-design.md) | Search |
| Blast radius | [blast-radius-design.md](blast-radius-design.md) | Blast Radius |
| Program slicing | [program-slicing-design.md](program-slicing-design.md) | Program Slicing |
| Taint analysis | [taint-analysis-design.md](taint-analysis-design.md) | Taint Analysis |
| CFG | [cfg-design.md](cfg-design.md) | CFG / PDG Analysis |
| PDG | [pdg-design.md](pdg-design.md) | Dataflow |
| Dominance | [dominance-design.md](dominance-design.md) | Dataflow → Dominator Tree |
| GQL | [gql-design.md](gql-design.md) | Graph Visualization (+ Query Guide) |
| Community query & naming | [community-query-and-naming-plan.md](community-query-and-naming-plan.md) | Graph Visualization (legend) + GQL |
| Hybrid CPG (two-resolution) | [hybrid-cpg-plan.md](hybrid-cpg-plan.md) | CLI/HTTP agent-first (`cpg`); dashboard optional later |
| Graph metrics | [graph-metrics-design.md](graph-metrics-design.md) | Functions |
| Migration planner | [migration-planner-design.md](migration-planner-design.md) | Migration |
| CI policy checks | [ci-policy-checks-design.md](ci-policy-checks-design.md) | CLI-first (blast scores in dashboard) |

## Screenshots

PNG assets live under [`docs/images/design/`](../images/design/) per feature subdirectory.

Regenerate after UI changes:

```bash
cd dashboard && npm run build
cargo build --release
rgctl -r /path/to/gbuilder discover . --with-cfg --with-security --with-taint
rgctl -r /path/to/gbuilder serve --port 8080

DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-design-screenshots.mjs
```

The capture script **selects functions from the sidebar** and triggers the right action per tab (Load CFG, Compute slice, dominator view, etc.). Override symbols for other repos:

```bash
CAPTURE_FN_BLAST=MyService \
CAPTURE_FN_CFG=MyService \
CAPTURE_FN_DATAFLOW=MyService \
CAPTURE_FN_SLICE=MyService \
CAPTURE_SLICE_LINE=42 \
CAPTURE_SLICE_VAR=orderId \
CAPTURE_FN_TAINT=handleRequest \
DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-design-screenshots.mjs
```

Migration-only extras (preset/tooltip shots):

```bash
DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/capture-migration-screenshots.mjs
```

Set `DESIGN_DOCS_SCREENSHOT_DIR` or `MIGRATION_DOCS_SCREENSHOT_DIR` to override output paths.

## User-facing companions

| Audience | Doc |
|----------|-----|
| Concepts | [Introduction.md](../Introduction.md) |
| Commands | [user-guide.md](../user-guide.md) |
| Dashboard walkthrough | [dashboard-user-guide.md](../dashboard-user-guide.md) |
| Migration workflow | [building-migration-plan.md](../building-migration-plan.md) |
