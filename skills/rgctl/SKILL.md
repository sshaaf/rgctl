---
name: rgctl
description: >-
  Answer structural questions about a codebase using the rgctl CLI graph
  (architecture, communities, call relationships, blast radius, data-flow
  slices, CPG, semantic search, migration, Konveyor Kantra rules,
  CI gates). Use when the user asks how code is connected, what calls what,
  impact of changing a symbol, where data flows, migration rule violations,
  Konveyor/quarkus/spring targets, repo structure/hotspots, or when `.rgctl/`
  exists — treat natural-language codebase questions as rgctl queries first.
rgctl-managed: true
---

# rgctl

Answer **structural** questions from a pre-built code knowledge graph instead of reading whole files into context. Spawn **`rgctl -f json`** subprocesses (or use foreground **`rgctl serve`** for repeated HTTP queries). Artifacts live at **`{repo}/.rgctl/`** after `discover`.

## When to Use This Skill

Use rgctl when the user asks:
- **Architecture questions** — "What calls X?", "Where is the checkout flow?", "What communities exist?"
- **Impact analysis** — "What breaks if I change this function?"
- **Data flow** — "Where does this variable flow?", "Trace this tainted input"
- **Migration planning** — "Generate a migration roadmap"
- **Konveyor / Kantra rules** — "What Quarkus migration rules apply?", "List rules for target X", "What violations did Kantra find?"
- **Hotspots** — "What are the most central/risky functions?"
- **Subsystem mapping** — "Which module owns feature X?"

If `.rgctl/` exists, prefer rgctl queries over reading files.

## CLI for agents

```bash
export REPO=/path/to/repo
cd "$REPO" && rgctl discover .          # or: rgctl -r "$REPO" discover
rgctl -r "$REPO" -f json <command> …
```

**Critical:** Parse `schema_version` + payload from **stdout**. **Never use `2>/dev/null`** — it swallows rgctl errors.

For interactive exploration, optional: `rgctl serve --open` (dashboard). Agents should still spawn CLI structured verbs (`find` / `callers` / `relations` / `inventory` / …).

Legacy daemon cache under `~/.rgctl/cache/` is obsolete; run `rgctl discover .` in the repo to build `{repo}/.rgctl/`.

## Agent Loop

```text
1. USER PROMPT     → natural language (not a CLI string)
2. SUBPROCESS      → rgctl -f json <structured command>
3. GRAPH FACTS     → parse schema_version + payload
4. LLM REASONING   → summarize using "what to report" guidelines
5. ACTION          → edit / plan / check — re-query if graph may be stale
```

**Prerequisite (once per repo):** `discover` (and `semantic index` when using search). Deep analysis (`cpg`, `inspect`, slice/taint) needs `discover --with-cfg`.

## What to Do When Invoked

1. **Help-only** — If user only wants help/command list → print workflow table below and **stop** (no discover, no queries)
2. **Fast path (existing index)** — If `.rgctl/` exists **and** request is a structural question (not rebuild) → **do not re-run discover**. Route via workflow table; use CLI `-f json`. **Exception — vuln / OSV / OpenVEX:** if reachability, sink-first taint, or `vuln analyze` is in scope and CFG was not built (`cpg status` / missing CFG archive), re-run `discover . --with-cfg` (add `--with-taint` for PDG-backed confidence) before P4–P6
3. **No index** — Run `cd "$REPO" && rgctl discover .` or `rgctl -r "$REPO" discover` (do **not** use `-r REPO discover .` — the `.` ignores `-r`). Add flags only when needed; for vuln scans use `--with-cfg` (see vuln workflow)
4. **Natural-language routing** — Map utterance with workflow table. Do not ask user to rephrase into CLI unless disambiguation required
5. **Summarize** — Report key facts, not raw JSON dumps
6. **Stop conditions** — Pure code-edit/debug with no structural need → do not force rgctl

**Relationship questions** (e.g., "relationship between X and Y"): resolve symbols → bounded `CALLS`/`DEPENDSON` traversal → answer in plain language (hops, shared neighbors, files). If no direct path but asymmetric dependency, fall back to `blast-radius` on each.

## Workflow Families

### 1. Discovery & Indexing

**When:** First use, or after major changes

| User Intent | CLI Command |
|-------------|-------------|
| Build graph index | `cd repo && discover .` or `rgctl -r PATH discover` |
| Build semantic index | `semantic index` |
| Check CFG readiness | `cpg status` |
| Full staged pipeline | `discover . --full` |

**Common flags:**
- `--with-cfg` — Enable CFG/PDG (for slice, inspect, cpg)
- `--with-dashboard` — Build dashboard bundle
- `--export-migration-hints` — Generate migration plan
- `--with-security --with-taint` — Security scanning
- `--with-kantra` — Konveyor Kantra rule eval + rules graph index (embedded catalog by default)

**See:** [Discovering and Indexing Guide](../../docs/guides/discovering-and-indexing.md)

### 1b. Konveyor Kantra rules (`--with-kantra`)

| User Intent | CLI Command |
|-------------|-------------|
| Evaluate migration rules | `discover . --with-kantra` or `rules run ./rules/` |
| Filter by migration target | `discover . --with-kantra --kantra-target quarkus` / `rules run ./rules/ --target quarkus` |
| CI / custom ruleset | `discover . --with-kantra --kantra-rules PATH` / `rules run PATH` |
| Index rules only | `discover . --with-kantra --kantra-index-only` |
| List indexed rules | `find --type kantrarule --limit 50` / `inventory --by type` |
| Rule → code links | `relations --edge violates --from-type kantrarule` |
| Read violations artifact | `.rgctl/kantra_findings.json` |

**See:** [User guide — Kantra](../../docs/user-guide.md#kantra-migration-rules---with-kantra), [JSON — kantra_findings](../../docs/json-api.md#kantra_findingsjson)

### 2. Query & Search

**Prefer structured verbs** (mmap; no Cypher). Parse `-f json` from **stdout** (`schema_version`); never `2>/dev/null`.

| User Intent | CLI Command |
|-------------|-------------|
| Session / index freshness | `status` |
| Schema / counts (incl. zeros) | `inventory --by type` or `inventory --by edge` |
| Import prefix census | `inventory --by import-prefix` |
| Count functions | `find --type function --count-only` |
| Find by name/type | `find "User*" --type class --limit 50` |
| Suffix scan (MDB / Remote) | `find '*MDB*' --type class` |
| Classes with annotation | `find --annotation @MessageDriven --type class` |
| javax import worklist | `find "import javax*" --type import --scope <pkg>` |
| Annotation pairs (seedless) | `relations --edge annotatedwith --from-type function --to-type annotation --scope <pkg>` |
| Find callers/callees | `callers <Symbol> --depth 1` / `callees <Symbol>` |
| Outside callers of a module | `callers <Symbol> --scope <pkg> --scope-mode outside` |
| EXTENDS / IMPLEMENTS inventory | `relations --edge extends --from-type class` (omit SYMBOL) |
| Natural-language search | `semantic query "checkout flow"` |
| List communities | `communities list` |
| Subsystem ownership | `semantic query "X" --scope community` |
| Refresh community labels | `communities label --write` |
| Community census | `inventory --by community` |

**Migration probe order:** `status` → `inventory --by import-prefix` → `find --annotation …` / suffix globs → `rules run` / `--with-kantra` → `callers InitialContext`.

**Complexity honesty:** exact name = hash index; prefix/`*mid*`/`--scope` may scan keys/columns until better indexes land. Module re-index is still a strong speed lever. Annotation **arguments** (e.g. `@Path("/x")`) need `--show-attributes` when `annotation_args.json` is present.
**See:** [Command Encyclopedia](references/command-encyclopedia.md) (find/callers/relations/inventory/status), [Semantic Search Guide](../../docs/guides/semantic-search.md)

### 3. Impact & Safety

| User Intent | CLI Command |
|-------------|-------------|
| Blast radius | `blast-radius <Symbol> --depth N` |
| Policy check (full codebase) | `check --policy-file policy.json` |
| Policy check (one symbol) | `blast-radius <Symbol> --policy-file policy.json` |
| OSV / CVE / OpenVEX | `discover . --with-cfg` then `vuln triage` → `deps check` → `vuln analyze` (see vuln workflow) |

**Vuln scans:** index with `--with-cfg` before sink-first taint / blast classify / exploitability VEX; add `--with-taint` when PDG-backed confidence is required. Plain discover is enough only for triage + deps-only early exit.

**See:** [Blast Radius Guide](../../docs/guides/blast-radius-analysis.md), [CI Policy Guide](../../docs/guides/ci-policy-checks.md), vuln workflow in [references/workflows.md](references/workflows.md)

### 4. Metrics & Analysis

| User Intent | CLI Command |
|-------------|-------------|
| PageRank hotspots | `metrics --pagerank` |
| Betweenness bridges | `metrics --betweenness` |
| Community stats | `metrics --communities` |

### 5. Code Analysis (CFG/PDG/Slicing)

| User Intent | CLI Command |
|-------------|-------------|
| CFG/PDG status | `cpg status` |
| Field mutations | `cpg mutations --type Foo` |
| Data flow trace | `cpg flows FILE --line N --variable V --function F` |
| Program slice | `slice FILE --line N --variable V --function F` |
| CFG inspection | `inspect <Symbol> cfg` |
| PDG inspection | `inspect <Symbol> pdg --edge-layer data` |

Needs `discover --with-cfg`. `--function` is method name, not class.

### 6. Export & Visualization

| User Intent | CLI Command |
|-------------|-------------|
| Export graph | `export --export-format graphviz --export-output OUT` |
| CPG export | `cpg export --format graphson --output cpg.json` |
| HTTP dashboard | `serve --open` |

## Natural Language → Command Routing

| User Says | Command |
|-----------|---------|
| "Generate migration plan" | `discover --export-migration-hints` → `.rgctl/migration_plan.json` |
| "Konveyor / Kantra violations" | `discover . --with-kantra` → `.rgctl/kantra_findings.json` |
| "Bottlenecks / hotspots" | `metrics --pagerank` |
| "Where is checkout flow?" | `semantic query "checkout flow" --limit 10` |
| "Impact if I change X" | `blast-radius X --depth 2` |
| "Are we affected by this CVE / OSV?" | `discover . --with-cfg` → `vuln triage` / `deps check` / `vuln analyze` (vuln workflow) |
| "Validate against policy" | `check --policy-file policy.json` |
| "Who calls X" | `callers X --depth 2` (impact → `blast-radius X`) |
| "javax imports / annotations" | `find "import javax*" --type import`; `relations --edge annotatedwith --from-type function --to-type annotation` |
| "Where is X mutated?" | `cpg mutations --type X --exclude-ctors` |

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| No `.rgctl/` in repo | Run `cd repo && rgctl discover .` |
| slice/inspect/cpg fails | Re-discover with `--with-cfg` |
| vuln analyze / sink-first taint weak (`cfg_available=false`) | Re-discover with `--with-cfg` (add `--with-taint` for PDG confidence) |
| semantic query fails | `semantic index` |
| Ambiguous symbol | Add `--class` or `--file` on callers/find |
| `check` exit 1 | Report violations (JSON still on stdout) |
| find/relations empty | Run `inventory --by type` / `--by edge` (zeros mean unpopulated schema); check `--scope` |
| Name glob returns 0 | Try `semantic query` / `communities list` / broader `find '*X*'` types |

## Artifacts

All paths under **`{repo}/.rgctl/`**:

| Path | Content |
|------|---------|
| `graph.snapshot.bin` | Main graph snapshot |
| `semantic_index.bin` | Semantic index |
| `migration_plan.json` | Migration roadmap |
| `kantra_findings.json` | Kantra violations (`--with-kantra`) |
| `dashboard/` | Dashboard bundle |
| `analysis/` | CFG/PDG archives |

## Usage Globals

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" discover
rgctl -r "$REPO" -f json <command> …
```

**Globals:** `-f json` (agents), `-r` / `--repo`, `-o` output file, `-d` / `--db` (custom graph cache path)

## Reference Files

- **[Command Encyclopedia](references/command-encyclopedia.md)** — Full command reference
- **[Workflows](references/workflows.md)** — Worked scenarios
- **[Communities & Policy](references/communities-and-policy.md)** — CI policy checks

## External Documentation

- [User Guide](../../docs/user-guide.md) — Complete CLI tutorial
- [JSON API](../../docs/json-api.md) — Schema specifications
- [Agent Recipes](../../docs/agent-recipes.md) — Copy-paste recipes
- [USER_AGENTS_TEMPLATE.md](../../docs/agents/USER_AGENTS_TEMPLATE.md) — paste into consumer repos
- [AGENTS.md](../../AGENTS.md) — contributor agent README (rgctl source tree)
- [Policy Format](../../docs/policy-format.md) — CI policy schema

## Installation

```bash
rgctl install --skill --tools cursor,claude,codex,antigravity,agents
```

Installs the single skill `rgctl` (with `references/`) for each selected adapter. Omitting `--tools` installs **cursor, claude, codex, agents, antigravity**; use `--tools all` for the full registry. See [docs/guides/agent-skill.md](../../docs/guides/agent-skill.md). Scenario prose lives in `references/workflows.md`.
