# rgBuilder

**A code knowledge graph built for LLM agents — accurate answers, minimal tokens, maximum speed.**

[![CI](https://github.com/sshaaf/rgBuilder/actions/workflows/ci.yml/badge.svg)](https://github.com/sshaaf/rgBuilder/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/sshaaf/rgBuilder?display_name=tag&label=release)](https://github.com/sshaaf/rgBuilder/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![User Guide](https://img.shields.io/badge/docs-User%20Guide-0A7EA4)](docs/user-guide.md)
[![Rust](https://img.shields.io/badge/Made%20with-Rust-orange?logo=rust)](https://www.rust-lang.org/)

**Try it now:** [Installation](docs/installation.md) · [User Guide](docs/user-guide.md) (index → query) · [Download latest release](https://github.com/sshaaf/rgBuilder/releases/latest) · [Agent skill](skills/rgbuilder/SKILL.md) · [AGENTS.md](AGENTS.md) for LLM workflows

AI coding agents default to reading files sequentially. That burns context, misses structure, and produces confident wrong answers about impact and dependencies. **rgBuilder indexes the whole repository once** into a rich graph with pre-computed **reachability**, then serves **compact, deterministic query results** — so agents (and humans) get the right slice of the codebase without loading it into the prompt.

## Demo

Quick cli tour:

https://github.com/user-attachments/assets/6f355d16-dd6f-43e2-baad-9d4372b84540


Dashboard tour (optional visualization):

https://github.com/user-attachments/assets/81328399-dca1-4232-8edc-2a23563c3451

---

## Built for agents

**Goal:** make LLM-assisted development **more accurate** while **using fewer tokens**.

| Without rgBuilder | With rgBuilder |
|------------------|---------------|
| Agent reads dozens of files to guess dependencies | Agent calls `blast-radius Symbol` → structured impact JSON |
| “What calls this?” requires search + inference | `gql` returns exact graph matches |
| Migration planning from partial context | **Migration planner** — package roadmap, dual ordering, tunable scores, interactive graph |
| Repeated file dumps every turn | One `discover`, then cache-backed queries via CLI or `-f json` |

rgBuilder answers **reachability and relation questions deterministically** from the indexed graph. The LLM reasons on **summaries and facts**, not raw repo grep — fewer tokens, less hallucination, faster turns.

**Primary outputs for agents:** `-f json` on `discover`, `gql`, `blast-radius`, `metrics`, `semantic`, `communities`, `cpg`, `check`, `slice`, and `inspect`. File export uses `export --export-format` (not stdout JSON). See **[JSON API](docs/json-api.md)**.

---

## Where most tools stop

Most codebase tools stop at **text search**, **file trees**, or a **shallow call graph**. rgBuilder goes further — compiler-grade structure and security analysis, pre-computed at index time, queryable in milliseconds. That is what makes agent answers trustworthy.

| Feature | What it gives you | Design doc |
|---------|-------------------|------------|
| **Semantic search** | **Natural-language and keyword search** over functions — **vocab** default, optional **code-daemon** / **hash**, Hamming retrieval, late fusion with blast/PageRank/sketches | [semantic-search-design.md](docs/design/semantic-search-design.md) |
| **Blast radius** | Pre-computed **reachability** over the call graph — upstream impact, scores, policy gates, sub-second on large repos | [blast-radius-design.md](docs/design/blast-radius-design.md) |
| **Program slicing** | **Backward / forward slice** — only the statements that affect (or are affected by) a line and variable | [program-slicing-design.md](docs/design/program-slicing-design.md) |
| **Taint analysis** | **Source → sink** flows (HTTP params → SQL, shell, render, …) with sanitizer awareness | [taint-analysis-design.md](docs/design/taint-analysis-design.md) |
| **CFG** | **Control-flow graph** per function — branches, loops, executable paths | [cfg-design.md](docs/design/cfg-design.md) |
| **PDG** | **Program dependence graph** — data and control deps between statements; foundation for slice and taint | [pdg-design.md](docs/design/pdg-design.md) |
| **Dominance** | **Dominator trees** and frontiers — the same structures compilers use for advanced analysis | [dominance-design.md](docs/design/dominance-design.md) |
| **Hybrid CPG** | **Unified façade** over L_repo CALL graph + L_proc CFG/PDG — mutations, flows, calls (`cpg`) | [hybrid-cpg-plan.md](docs/design/hybrid-cpg-plan.md) |
| **GQL** | **Graph query language** over 30+ relation types — inventory, call chains, patterns | [gql-design.md](docs/design/gql-design.md) |
| **Graph metrics** | **PageRank**, **betweenness**, **communities** (label propagation) on the live call graph | [graph-metrics-design.md](docs/design/graph-metrics-design.md) |
| **Named communities** | **`communities` CLI** — list / refresh heuristic labels over label-propagation clusters | [graph-metrics-design.md](docs/design/graph-metrics-design.md) |
| **Migration planner** | **Package-level roadmap** — PageRank + harmonic centrality − blast radius; dependency-aware schedule and priority rank | [migration-planner-design.md](docs/design/migration-planner-design.md) |
| **CI policy checks** | **`check`** — fail builds when blast-radius rules are violated on touched symbols | [ci-policy-checks-design.md](docs/design/ci-policy-checks-design.md) |

All of the above share one index: run [`discover`](docs/Introduction.md#indexing-the-repository-discover) once (use [`discover --with-cfg`](docs/Introduction.md#indexing-the-repository-discover) / `--with-taint` for CFG/PDG/taint archives; add `--export-migration-hints` when you need a migration plan JSON). **Semantic search** is opt-in: `rgctl semantic index` after discover. Explore in the **CLI** and pipe **JSON** to agents. An optional browser UI exists after `discover --with-dashboard` — see [dashboard user guide](docs/dashboard-user-guide.md) if you want it.

**Deep dive → [Introduction](docs/Introduction.md) · [User Guide](docs/user-guide.md) · [Feature designs](docs/design/README.md) (contributors)**

---

## Speed by design

rgBuilder is **async and parallel by design** — discovery walks the tree, parses languages concurrently, and builds analytics on the graph in parallel (Rayon + Tokio throughout the pipeline).

- **Full discovery in seconds** on typical repos (not minutes of ad-hoc agent exploration)
- **Reachability compressed** — enterprise-scale call graphs stored in compact on-disk snapshots, not gigabytes in RAM
- **HTTP `serve`** — dashboard + `/api/query` on port 8080; optional `serve --daemon` socket for blast-radius warm path

Index once → query many times. That is the agent workflow.

```text
  Agent / script / human
           │
           ▼
    rgctl gql | blast-radius | metrics | semantic | export -f json
           │
           ▼
  .rgbuilder/  ← graph snapshot + reachability engine + indexes
           ▲
           │
      discover .     ← async parallel index (seconds)
           ▲
           │
     Your repository
```

---

## Code understanding and migrations

Use the features above together for **migration and modernization** work:

- **Migration planner** — package-level graph, tunable scoring presets, dependency-aware schedule vs. priority rank
- **Blast radius** + **metrics** — see fan-in and architectural hotspots before moving a service or framework
- **GQL** + **export** — inventory symbols and ship subgraphs to downstream tools
- **Slice** + **taint** — validate data-flow assumptions agents often get wrong
- **`check`** — enforce blast-radius policy in CI while agents (or humans) land changes

### Migration planner

After deep discover + `--export-migration-hints`, read `.rgbuilder/migration_plan.json` (agents) or optionally open a dashboard **Migration** tab if you also passed `--with-dashboard`:

*Scoring and package ordering; community coloring uses label propagation — see [graph metrics naming](docs/design/graph-metrics-design.md#31-community-detection-naming).*

- **Package macro graph** — aggregates functions into path-derived package labels (Java package paths, Rust/C `/src/` modules)
- **Dual ordering** — **scheduled step** (Kahn topological sort, callee before caller) and **priority rank** (score-only)
- **Scoring** — `Priority = α·PageRank + β·Harmonic − γ·Blast`; presets include Hybrid Default, Foundational First, Dense Cluster Extraction, Risk Mitigation
- **CLI export** — `--export-migration-hints` writes a preset-tuned plan (default `.rgbuilder/migration_plan.json`); `--with-dashboard` additionally copies UI assets under `.rgbuilder/dashboard/`

```bash
rgctl discover . --with-cfg --with-security --with-taint --with-harmonic --export-migration-hints
# optional UI: add --with-dashboard, then: rgctl serve --open
rgctl serve   # http://127.0.0.1:8080/ → Migration tab
```

Design → **[Migration planner design](docs/design/migration-planner-design.md)** · Workflow → **[Building a migration plan](docs/building-migration-plan.md)**  
All feature designs → **[docs/design/](docs/design/README.md)**

### Community detection naming

rgBuilder does **not** run the Leiden algorithm today. What ships is **label propagation** (Raghavan et al., 2007) with Newman modularity scoring, plus hub stripping and deterministic tie-breaking. Docs/UI still say “Louvain” in places (`louvain_community_id`, migration layout), and `TASK_PLAN.md` lists Leiden as planned but unimplemented.

| Name in repo | What it actually is |
|--------------|---------------------|
| `CommunityDetector` | Label propagation on `Calls` + `Uses` |
| “Louvain” in dashboard/migration | Majority vote of label-propagation ids |
| Leiden (task 2.1.1) | Not implemented |

Full detail → **[Graph metrics — community naming](docs/design/graph-metrics-design.md#31-community-detection-naming)**.

Walkthrough on the in-tree Spring Boot fixture → **[ecommerce-java example](docs/user-guide.md#3-example-project-ecommerce-java)** (User Guide).

**Research map** — which papers rgBuilder implements, which inspire the roadmap, and where to propose changes → **[Further reading](docs/further-reading.md#research-foundations-in-rgbuilder)**.

---

## Quick start

**Install** from [GitHub Releases](https://github.com/sshaaf/rgBuilder/releases) or build from source (full guide: [Installation](docs/installation.md)):

```bash
git clone https://github.com/sshaaf/rgBuilder.git
cd rgBuilder
git lfs pull   # only if you use `semantic index --embedder code-daemon` (~206 MB)
cargo build --release
```

**Discover** (build the graph + reachability caches):

```bash
git clone https://github.com/konveyor-ecosystem/coolstore.git
cd coolstore
rgctl discover .
# agent-friendly telemetry:
rgctl -f json discover . | jq '.metrics'
```

**Query** (compact answers instead of file dumps):

```bash
# Graph inventory for the agent
rgctl -f json gql 'MATCH (n:Function) RETURN n LIMIT 10'

# Impact — critical before the agent edits a symbol
rgctl -f json blast-radius ShoppingCartService

# Hotspots — where migration/refactor pain concentrates
rgctl -f json metrics --pagerank --communities

# Package migration roadmap (graph + plan JSON for agents)
rgctl discover . --with-cfg --with-security --with-taint --with-dashboard --with-harmonic --export-migration-hints
```

Concepts → **[Introduction](docs/Introduction.md)** · Commands → **[User Guide](docs/user-guide.md)**

Example deep-analysis commands (after `discover --with-cfg`):

```bash
rgctl inspect checkout cfg              # CFG / PDG / dominance (function symbol)
rgctl slice src/Foo.java --line 42 --variable x
rgctl slice src/Foo.java --line 10 --variable req --taint
rgctl semantic index                    # default vocab; optional: --embedder code-daemon|hash
rgctl -f json semantic query "checkout flow" --limit 10
```

---

## What the **R** stands for

| **R** | Meaning |
|-------|---------|
| **Rust** | Memory-safe, predictable performance at scale — the foundation for parsing large monorepos without blowing the heap |
| **Reachability** | Pre-computed call reachability (sparse bitsets, not multi‑GB dense matrices) so “what breaks if I change this?” stays sub-second |
| **Rich** code graph | 30+ typed relations — CALLS, IMPORTS, CONTAINS, IMPLEMENTS, and more — not just files and folders |

Together: **rgBuilder** is the **reachability builder** — it constructs the graph and the compressed reachability engine agents need for trustworthy structural reasoning.

Algorithm and complexity details: crate READMEs under `crates/rgbuilder-analysis/` and [CLI I/O sanity QE](docs/cli-io-sanity-qe.md) for automated perf gates.

---

## Command reference

Quick links into **[Introduction](docs/Introduction.md)** — see [Where most tools stop](#where-most-tools-stop) for the differentiators.

| Command | Introduction |
|---------|----------------|
| `discover` | [Indexing](docs/Introduction.md#indexing-the-repository-discover) |
| `gql` | [Graph queries](docs/Introduction.md#graph-queries-gql) |
| `blast-radius` | [Blast radius](docs/Introduction.md#blast-radius-change-impact) |
| `slice` | [Program slicing](docs/Introduction.md#program-slicing) · [Taint](docs/Introduction.md#taint-analysis) |
| `inspect` | [CFG, PDG, dominance](docs/Introduction.md#cfg-pdg-and-dominance-deep-structure) |
| `metrics` | [Graph metrics](docs/Introduction.md#graph-metrics-architecture-hotspots) |
| `semantic` | [Semantic search](docs/Introduction.md#semantic-search-opt-in) (opt-in index + query) |
| `communities` | [Graph metrics](docs/Introduction.md#graph-metrics-architecture-hotspots) · [User Guide](docs/user-guide.md#6-query-the-graph-with-gql) |
| `cpg` | [Hybrid CPG](docs/Introduction.md#hybrid-cpg-mutations-and-flows) |
| `export` | [Export](docs/Introduction.md#export-and-sharing) |
| `check` | [CI policy](docs/Introduction.md#ci-policy-checks) |
| `serve` | [HTTP server](docs/Introduction.md#http-server-serve) |

**Dashboard** — visual exploration after `discover --with-dashboard` (`.rgbuilder/dashboard/`). See **[Feature designs](docs/design/README.md)** for per-tab engineering docs.  
**Migration export** — `discover --export-migration-hints` (alias `--export-migration-plan`; optional `--migration-preset`, `--migration-order scheduled|priority`).  
**Languages** — nine Tier 1 languages (Rust, Python, Java, Go, TypeScript, JavaScript, C#, C, C++) plus config/IaC plugins and **markdown** (`.md` / `.mdx` docs context). See [Languages](docs/languages.md) and [Markdown context](docs/markdown-context.md).

---

## Documentation

| Document | For |
|----------|-----|
| **[Documentation index](docs/README.md)** | Map of all docs by persona |
| **[Installation](docs/installation.md)** | Install rgctl, choose operating mode (CLI / HTTP / MCP), verify setup |
| **[Introduction](docs/Introduction.md)** | Concepts — graph, reachability, capability map |
| **[User Guide](docs/user-guide.md)** | ecommerce-java fixture, every CLI command |
| **[Agent skill](skills/rgbuilder/SKILL.md)** | **Canonical agent playbook** — NL routing + CLI samples. Install with `rgctl install --skill` |
| **[AGENTS.md](AGENTS.md)** | Minimal agent contract (points at skill) |
| **[Agent recipes](docs/agent-recipes.md)** | Copy-paste automation workflows |
| **[JSON API](docs/json-api.md)** | Parse `-f json` payloads + field catalogs |
| **[HTTP API](docs/http-api.md)** | `rgctl serve` → `/api/query` and `/api/semantic/*` |
| **[Policy format](docs/policy-format.md)** | `check` / blast policy JSON |
| **[Languages](docs/languages.md)** | Supported languages and tiers |
| **[Markdown context](docs/markdown-context.md)** | Doc graph — headings, links, doc→code GQL |
| **[Further reading](docs/further-reading.md)** | Research implemented vs inspired |
| **[CLI I/O sanity QE](docs/cli-io-sanity-qe.md)** | Subprocess JSON contract and release perf gates *(contributors)* |
| **[Feature designs](docs/design/README.md)** | Engineering design docs *(contributors)* |
| **[Migration planner design](docs/design/migration-planner-design.md)** | Package graph, scoring, ordering *(contributors)* |
| **[Building a migration plan](docs/building-migration-plan.md)** | End-to-end migration workflow |
| **[Dashboard user guide](docs/dashboard-user-guide.md)** | Optional browser UI |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | Dev setup and PR expectations |
| **[Releasing](docs/releasing.md)** | Tags and GitHub Releases *(contributors)* |

---

## Development

```bash
cargo test
cargo build --release
# See CONTRIBUTING.md for dashboard build and golden-repo checks
```

---

## License

MIT — see [LICENSE](LICENSE).
