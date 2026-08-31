# Dashboard user guide

Interactive browser UI for exploring a repository after `discover`. This guide is for **end users**; engineering detail lives in [dashboard-design.md](dashboard-design.md).

**CLI equivalents:** each tab’s **Query Guide** panel lists matching `rgctl` commands.

---

## Prerequisites

1. Index the repo (from repo root):

```bash
cd /path/to/your/repo
rgctl discover . --with-dashboard          # graph + dashboard bundle
# or
rgctl discover . --with-cfg --with-security --with-taint --with-dashboard    # CFG, PDG, taint + dashboard
```

The dashboard bundle is written to `{repo}/.rgctl/dashboard/` when you pass `--with-dashboard` during `discover`.

2. Open the dashboard over **HTTP** (required for WASM):

```bash
# Option A — integrated server (dashboard + query API; recommended)
rgctl -r /path/to/your/repo serve --open

# Option B — static files only (serve dashboard dir directly)
cd /path/to/your/repo/.rgctl/dashboard && python3 -m http.server 8765
# open http://localhost:8765/
```

Do **not** open `index.html` via `file://` — the graph worker cannot load `graph_payload.bin`.

---

## Layout

| Area | Description |
|------|-------------|
| **Stat cards** | Node/edge/function counts from `manifest.json` |
| **Tab bar** | Graph, Search, Functions, CFG, Dataflow, Slice, Blast, Taint, Migration, Query Guide |
| **Tab panels** | Collapsible help text per tab (click header to expand) |
| **Notification menu** | Engine/WASM status, manifest errors |

Screenshot placeholders (capture with `dashboard/scripts/capture-migration-screenshots.mjs` pattern → `docs/images/dashboard/`):

- `dash-overview.png` — full shell with stat cards
- `dash-query-guide.png` — Query Guide tab

---

## Tab guide

### Search

- Natural-language and keyword search over indexed functions (default **vocab**; optional **code-daemon** / **hash** via CLI).
- **Late fusion** (on by default) blends Hamming similarity with blast score, PageRank, name overlap, and token-bloom sketches.
- Requires `rgctl semantic index` (choose embedder at index time) and **`rgctl serve`** (HTTP API at `/api/semantic/*` — not static-only hosting). Restart `serve` after rebuilding the index.
- Status badge shows `model_id` (e.g. `vocab-accumulate-v1`).
- **CLI:** `semantic index`, `semantic query "…"` (`--keyword-and`, `--no-fusion`, `--expand neighbors`)

### Graph

- **Package metagraph** — zoomable WebGL view of communities / packages.
- **Community names** — heuristic labels (package path, dominant tokens, infrastructure hubs), not anonymous `Community N` when inference succeeds. Refresh with `rgctl communities label --write`.
- **Drill-down** — click a package node to expand member functions (WASM `expand`).
- **Filters** — search box, community filter, function/class type mask.
- **CLI:** `gql --macro-name all_communities`, `communities list`, `export`, `metrics --communities`

### Functions

- Sortable table: PageRank, betweenness, harmonic, blast score.
- WASM paginated list over the full function inventory.
- **CLI:** `gql --macro-name all_functions`, `metrics --pagerank`

### CFG

- Pick a function from the list; view control-flow blocks and dominance.
- **Large repos:** when per-function JSON is omitted (`archive_only`), a banner offers **Load CFG graph** — fetches one function from the CFG record pack on demand.
- **CLI:** `inspect <symbol> cfg`, `inspect <symbol> dom --frontiers`

### Dataflow

- PDG visualization and statement list; dominator tree mode.
- **Field mutations (CPG):** type filter (e.g. `ShoppingCart`), exclude constructors, click a hit to open that function and highlight the write line. Backed by `mutations_index.json` from `field_write.index.bin` (`discover --with-cfg --with-dashboard`).
- **CLI:** `inspect <symbol> pdg`, `cpg mutations --type ShoppingCart --exclude-ctors`, `slice ... --view pdg`

### Slice

- Enter file path, line, variable, direction; highlights affected lines.
- Requires `discover --with-cfg` / `--with-taint` and exported slice bundles.
- **CLI:** `slice <file> --line N --variable V --function <methodName>`

### Blast radius

- Summary cards use full-graph blast scores from discover.
- Caller table respects the **depth slider** (may differ from sidebar score).
- **CLI:** `blast-radius <symbol> --depth N`

### Taint

- Lists source→sink flows exported at discover time.
- **CLI:** `slice ... --taint` for on-demand trace at a line

### Migration

- Tune α/β/γ weights and presets; package graph + ordered table.
- Requires `discover --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints`.
- Screenshots: [design/README.md](design/README.md) (figures under `docs/images/design/`).
- **CLI:** `discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints`

### Query Guide

- Scrollable **CLI cookbook** organized by tab (prerequisites, commands, notes).
- Validated against gbuilder: `dashboard/scripts/validate-guide-cli-gbuilder.sh`
- Live GQL in the browser requires `rgctl serve` ([HTTP API](http-api.md)).

---

## Large repositories

| Symptom | Cause | Action |
|---------|-------|--------|
| CFG tab shows warning, no graph | `archive_only` mode (too many functions for inline JSON) | Click **Load CFG graph** per function |
| Slow first tab load | Large `graph_payload.bin` | Normal; WASM parses columnar snapshot once |
| Blank graph | Served over `file://` | Use `python3 -m http.server` or `rgctl serve` |

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| “Graph not found” / empty stats | Run `discover . --with-dashboard` from repo root, then `rgctl -r REPO serve --open` (not `file://`) |
| WASM engine error in notifications | Rebuild dashboard (`npm run build` in `dashboard/`) and re-run `discover --with-dashboard` |
| Stale data after git pull | Re-run `discover` (with `--with-dashboard` if using UI) |
| Semantic search empty / warning | `rgctl semantic index` then `rgctl serve --open` |
| Migration tab empty | `rgctl discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints` |

---

## See also

- [User Guide §15 — HTTP server](user-guide.md#15-http-server-serve--optional)
- [User Guide](user-guide.md)
- [HTTP API](http-api.md) — `rgctl serve` query endpoint
