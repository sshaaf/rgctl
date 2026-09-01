# Reachability Graph Control (rgctl)

**A code knowledge graph built for LLM agents — accurate answers, minimal tokens, maximum speed.**

> **rgctl** indexes your repository once, then answers reachability and structure questions in compact JSON — so coding agents use fewer tokens and make fewer confident mistakes.

AI coding agents default to reading files sequentially. That burns context, misses structure, and produces confident wrong answers about impact and dependencies. **rgctl indexes the whole repository once** into a rich graph with pre-computed **reachability**, then serves **compact, deterministic query results** — so agents (and humans) get the right slice of the codebase without loading it into the prompt.

<div align="center">
  <a href="https://youtu.be/SXxI-w9pOR0">
    <img src="https://markdown-videos-api.jorgenkh.no/youtube/SXxI-w9pOR0" alt="Watch the video" width="100%">
  </a>
</div>

---

## Built for agents

**Goal:** make LLM-assisted development **more accurate** while **using fewer tokens**. Anyone can use it directly via the CLI, or drop it into an IDE (Cursor, Aider, OpenHands, etc.) to give the model superhuman architectural awareness.

| Without rgctl | With rgctl |
| --- | --- |
| Agent reads dozens of files to guess dependencies | Agent calls `blast-radius Symbol` → structured impact JSON |
| “What calls this?” requires search + inference | `gql` returns exact graph matches |
| Migration planning from partial context | **Migration planner** — package roadmap, dual ordering, tunable scores |
| Repeated file dumps every turn | One `discover`, then queries via CLI `-f json` or HTTP `serve` |

The LLM reasons on **summaries and facts**, not raw repo grep — fewer tokens, less hallucination, faster turns. Primary agent outputs use `-f json` on `discover`, `gql`, `blast-radius`, `metrics`, `semantic`, and `slice`. See the **[JSON API](docs/json-api.md)**.

---

## Quick Start

**1. Install** from [GitHub Releases](https://github.com/sshaaf/rgctl/releases/latest) (binary **`rgctl`**) or build from source ([Installation docs](docs/installation.md)):

```bash
git clone https://github.com/sshaaf/rgctl.git
cd rgctl
git lfs pull   # only if you use `semantic index --embedder code-daemon` (~206 MB)
cargo build --release --bin rgctl

```

**2. Discover (Index your repo):**
Run this once to build the graph and reachability caches. Artifacts land in `{repo}/.rgctl/`.

```bash
cd your-project-repo
rgctl discover .  # Runs in seconds

```
For more details on commands and different options, see **[Command reference](docs/user-guide.md)**.
*(Upgrading from an old daemon install? `rgctl migrate-cache` copies `~/.rgctl/cache/{name}/.rgctl/` into the repo.)*

**3. Query (Ask the graph):**
Get compact, exact answers instead of file dumps:

```bash
# Graph inventory for the agent
rgctl -f json gql 'MATCH (n:Function) RETURN n LIMIT 10'

# Impact — critical before the agent edits a symbol
rgctl -f json blast-radius ShoppingCartService

# Advanced: Program slicing / taint analysis (requires `discover --with-cfg`)
rgctl slice src/Foo.java --line 42 --variable x

```

**🤖 Using with LLM IDEs?**
Simply point your AI assistant to our **[AGENTS.md](AGENTS.md)** file, or install the agent skill natively via `rgctl install --skill` to use the **[Agent skill](skills/rgctl/SKILL.md)** playbook for seamless IDE routing.

---

## Architecture & Speed

rgctl is **async and parallel by design** — discovery walks the tree, parses languages concurrently, and builds analytics on the graph in parallel using Rust (Rayon + Tokio).

The tool follows a fast, two-step model: **Index once → Query many times.**

```text
 1. Indexing (Run Once):
    Your Repository ──(rgctl discover)──> {repo}/.rgctl/ (Compact Caches)

 2. Querying (Run Many Times):
    LLM Agent ──(rgctl blast-radius)──> {repo}/.rgctl/ ──(JSON Facts)──> LLM Agent
                (or HTTP serve for /api/query)

```

**What the R stands for:**

* **Rust:** Memory-safe, predictable performance at scale without blowing the heap.
* **Reachability:** Pre-computed sparse bitsets keep “what breaks if I change this?” queries sub-second.
* **Rich graph:** 30+ typed relations (CALLS, IMPORTS, CONTAINS), not just files and folders.

*(Algorithm details: crate READMEs under `crates/rgctl-analysis/` and [CLI I/O sanity QE](docs/cli-io-sanity-qe.md) for automated perf gates.)*

---

## Where most tools stop

Most codebase tools stop at text search or a shallow call graph. rgctl goes further — compiler-grade structure and security analysis, pre-computed at index time.

| Feature | What it gives you | Design doc |
| --- | --- | --- |
| **Semantic search** | **Natural-language search** over functions — vocab, code-daemon, or hash. | [semantic-search-design.md](docs/design/semantic-search-design.md) |
| **Blast radius** | Pre-computed **reachability** — upstream impact, scores, policy gates. | [blast-radius-design.md](docs/design/blast-radius-design.md) |
| **Program slicing** | **Backward / forward slice** — statements affecting a line/variable. | [program-slicing-design.md](docs/design/program-slicing-design.md) |
| **Taint analysis** | **Source → sink** flows (HTTP params → SQL, shell) with sanitizer awareness. | [taint-analysis-design.md](docs/design/taint-analysis-design.md) |
| **CFG & PDG** | **Control-flow** & **Program dependence graphs** per function. | [cfg-design.md](docs/design/cfg-design.md) / [pdg-design.md](docs/design/pdg-design.md) |
| **Dominance** | **Dominator trees** — structures compilers use for advanced analysis. | [dominance-design.md](docs/design/dominance-design.md) |
| **Hybrid CPG** | **Unified façade** over CALL graph + CFG/PDG (`cpg`). | [hybrid-cpg-plan.md](docs/design/hybrid-cpg-plan.md) |
| **GQL** | **Graph query language** over 30+ relation types. | [gql-design.md](docs/design/gql-design.md) |
| **Graph metrics** | **PageRank, betweenness, communities** (label propagation). | [graph-metrics-design.md](docs/design/graph-metrics-design.md) |
| **Migration planner** | **Package-level roadmap** — dependency-aware schedule and priority rank. | [migration-planner-design.md](docs/design/migration-planner-design.md) |
| **Kantra migration rules** | **Konveyor rule evaluation** — embedded catalog, violations JSON, GQL `VIOLATES`, dashboard Migration Rules tab. | [user guide §4](docs/user-guide.md#kantra-migration-rules---with-kantra) · [rgctl-kantra](crates/rgctl-kantra/README.md) |
| **CI policy checks** | **`check`** — fail builds on blast-radius violations. | [ci-policy-checks-design.md](docs/design/ci-policy-checks-design.md) |

*(Deep dive → [Introduction](docs/Introduction.md) · [User Guide](docs/user-guide.md) · [Feature designs](docs/design/README.md))*

---

## Code Migrations & Advanced Analysis

rgctl ships with deep, enterprise-ready features for heavy modernization workloads.

* **Migration Planner:** Run `discover --with-cfg --with-security --with-taint --export-migration-hints` to generate a tunable, package-level `.rgctl/migration_plan.json`. This uses PageRank, harmonic centrality, and blast radius to prioritize what to move first. Read more in **[Building a migration plan](docs/building-migration-plan.md)** and the **[Migration planner design](docs/design/migration-planner-design.md)**.
* **Konveyor Kantra Rules:** For Java migrations, `discover --with-kantra` evaluates ~2.6k embedded migration rules. See [user guide §4](docs/user-guide.md#kantra-migration-rules---with-kantra) and [rgctl-kantra](crates/rgctl-kantra/README.md).
* **Community Detection:** Analyzes architectural hotspots using label propagation. Read the exact implementation details in **[Graph metrics — community naming](docs/design/graph-metrics-design.md#31-community-detection-naming)**.
* **Dashboard:** Add `--with-dashboard` during discovery to explore these metrics visually via `rgctl serve`. See the [dashboard user guide](docs/dashboard-user-guide.md).

*(Walkthrough on the in-tree Spring Boot fixture → **[ecommerce-java example](docs/user-guide.md#3-example-project-ecommerce-java)**. Research map for underlying papers → **[Further reading](docs/further-reading.md#research-foundations-in-rgctl)**).*

---

## Command Reference

| Command | User Guide Link |
| --- | --- |
| `discover` | [§4 Index with discover](docs/user-guide.md#4-index-with-discover) |
| `gql` | [§6 Query the graph with GQL](docs/user-guide.md#6-query-the-graph-with-gql) |
| `blast-radius` | [§7 Blast radius](docs/user-guide.md#7-blast-radius-change-impact) |
| `slice` | [§8 Program slicing and taint](docs/user-guide.md#8-program-slicing-and-taint) |
| `inspect` | [§9 Inspect CFG / PDG / dominance](docs/user-guide.md#9-inspect-cfg--pdg--dominance) |
| `metrics` | [§11 Graph metrics](docs/user-guide.md#11-graph-metrics) |
| `semantic` | [§12 Semantic search](docs/user-guide.md#12-semantic-search) |
| `communities` | [§6 GQL](docs/user-guide.md#6-query-the-graph-with-gql) · [§11 metrics](docs/user-guide.md#11-graph-metrics) |
| `cpg` | [§10 Hybrid CPG](docs/user-guide.md#10-hybrid-cpg-cpg) |
| `export` | [§13 Export](docs/user-guide.md#13-export-graph-projections) |
| `check` | [§14 CI policy check](docs/user-guide.md#14-ci-policy-check) |
| `serve` | [§15 HTTP server](docs/user-guide.md#15-http-server-serve--optional) |

**Languages supported:** Nine Tier 1 languages (Rust, Python, Java, Go, TypeScript, JavaScript, C#, C, C++) plus config/IaC plugins and markdown. See [Languages](docs/languages.md) and [Markdown context](docs/markdown-context.md).

---

## Documentation Directory

| Document | For |
| --- | --- |
| **[Documentation index](docs/README.md)** | Map of all docs by persona |
| **[Installation](docs/installation.md)** | Install rgctl, CLI / HTTP modes, verify setup |
| **[v0.4.9 release notes](docs/releases/v0.4.9.md)** | Kantra migration rules, CLI-first artifacts, daemon/MCP removed |
| **[v0.4.8 release notes](docs/releases/v0.4.8.md)** | Agent docs (historical — daemon era) |
| **[Introduction](docs/Introduction.md)** | Concepts — graph, reachability, capability map |
| **[User Guide](docs/user-guide.md)** | ecommerce-java fixture, every CLI command |
| **[Agent skill](skills/rgctl/SKILL.md)** | **Canonical agent playbook** — NL routing + CLI samples. |
| **[AGENTS.md](AGENTS.md)** | Minimal agent contract (points at skill) |
| **[Agent recipes](docs/agent-recipes.md)** | Copy-paste automation workflows |
| **[JSON API](docs/json-api.md)** | Parse `-f json` payloads + field catalogs |
| **[HTTP API](docs/http-api.md)** | `rgctl serve` → `/api/query` and `/api/semantic/*` |
| **[Policy format](docs/policy-format.md)** | `check` / blast policy JSON |
| **[CONTRIBUTING.md](CONTRIBUTING.md)** | Dev setup and PR expectations |
| **[Releasing](docs/releasing.md)** | Tags and GitHub Releases *(contributors)* |

*(For design docs, QE testing, and advanced implementation details, check the [Where most tools stop](#where-most-tools-stop) section above).*

---

## License

MIT — see [LICENSE](LICENSE).
