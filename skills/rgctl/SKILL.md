---
name: rgctl
description: >-
  Answer structural questions about a codebase using the rgctl CLI graph
  (architecture, communities, call relationships, blast radius, data-flow
  slices, CPG, semantic search, migration, CI gates). Use when the user asks
  how code is connected, what calls what, impact of changing a symbol,
  where data flows, repo structure/hotspots, or when `.rgctl/` exists —
  treat natural-language codebase questions as rgctl queries first.
---

# rgctl

Answer **structural** questions from a pre-built code knowledge graph instead of reading whole files into context. rgctl builds a queryable graph of your codebase and provides both MCP tools (for IDE integration) and CLI commands (for agents and CI).

## When to Use This Skill

Use rgctl when the user asks:
- **Architecture questions** — "What calls X?", "Where is the checkout flow?", "What communities exist?"
- **Impact analysis** — "What breaks if I change this function?"
- **Data flow** — "Where does this variable flow?", "Trace this tainted input"
- **Migration planning** — "Generate a migration roadmap"
- **Hotspots** — "What are the most central/risky functions?"
- **Subsystem mapping** — "Which module owns feature X?"

If `.rgctl/` exists, prefer rgctl queries over reading files.

## MCP vs CLI Decision

rgctl offers two interfaces:

### MCP Tools (Preferred in IDE)

If the host connected **`rgctl serve --mode mcp`**, prefer these **7 MCP tools**:

| Tool | Purpose | When to Use |
|------|---------|-------------|
| `rgctl_status` | Pipeline status, artifact readiness | Check if graph/CFG/semantic index is ready |
| `rgctl_query` | GQL queries + macros | Inventory, callers/callees, communities, relationships |
| `rgctl_search` | Semantic search | Natural-language "where is checkout flow?" |
| `rgctl_impact` | Blast-radius analysis | "What breaks if I change X?" |
| `rgctl_metrics` | PageRank, betweenness, communities | "What are the hotspots/bottlenecks?" |
| `rgctl_cpg` | CFG/PDG/slice/mutations/flows | Data-flow, field writes, loop hazards |
| `rgctl_check` | Policy validation | CI gates, architectural rules |

**MCP benefits:**
- One long-lived session (no cold starts)
- Auto-pipeline on connect (discover → CFG → semantic)
- Honest unreadiness (status JSON when artifacts still building)

**NOT MCP tools** (CLI only): `discover`, `semantic index`, `cpg export`, `communities label --write`, dashboard HTTP.

### CLI Commands (Agents & CI)

When MCP is not available, or for operations MCP doesn't support, spawn `rgctl -f json`:

```bash
export REPO=/path/to/repo
rgctl -r "$REPO" discover
rgctl -r "$REPO" -f json <command> …
```

**Critical:** Prefer `-f json` on **stdout**. Parse `schema_version` + payload from stdout. **Never use `2>/dev/null`** — it swallows rgctl errors and causes parse failures.

## Agent Loop

```text
1. USER PROMPT     → natural language (not a CLI string)
2. TOOL CALL       → MCP tool or rgctl -f json <command>
3. GRAPH FACTS     → parse schema_version + payload
4. LLM REASONING   → summarize using "what to report" guidelines
5. ACTION          → edit / plan / check — re-query if graph may be stale
```

**Prerequisite (once per repo):** `discover` (and `semantic index` when using search). Deep analysis (`cpg`, `inspect`, slice/taint) needs `discover --with-cfg`.

## What to Do When Invoked

1. **Help-only** — If user only wants help/command list → print workflow table below and **stop** (no discover, no queries)
2. **Fast path (existing index)** — If `.rgctl/` exists **and** request is a structural question (not rebuild) → **do not re-run discover**. Route via workflow table; use MCP tools or CLI `-f json`
3. **No index** — Run `cd "$REPO" && rgctl discover .` or `rgctl -r "$REPO" discover` (do **not** use `-r REPO discover .` — the `.` ignores `-r`). Add flags only when needed
4. **Natural-language routing** — Map utterance with workflow table. Do not ask user to rephrase into CLI unless disambiguation required
5. **Summarize** — Report key facts, not raw JSON dumps. See "what to report" under each command
6. **Stop conditions** — Pure code-edit/debug with no structural need → do not force rgctl

**Relationship questions** (e.g., "relationship between X and Y"): resolve symbols → bounded `CALLS`/`DEPENDSON` traversal → answer in plain language (hops, shared neighbors, files). If no direct path but asymmetric dependency, fall back to `blast-radius` on each.

## Workflow Families

Organized by user intent (like MCP's 7 tools):

### 1. Discovery & Indexing

**When:** First use, or after major changes

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| Build graph index | — | `cd repo && discover .` or `rgctl -r PATH discover` |
| Build semantic index | — | `semantic index` |
| Check pipeline status | `rgctl_status` | `cpg status` |

**Common flags:**
- `--with-cfg` — Enable CFG/PDG (for slice, inspect, cpg)
- `--with-dashboard` — Build dashboard bundle
- `--export-migration-hints` — Generate migration plan
- `--with-security --with-taint` — Security scanning

**See:** [Discovering and Indexing Guide](../../docs/guides/discovering-and-indexing.md)

### 2. Query & Search

**When:** Find code, explore architecture, map subsystems

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| Inventory functions | `rgctl_query` macro: `all_functions` | `gql --macro-name all_functions unused` |
| Find callers/callees | `rgctl_query` | `gql "MATCH (a)-[:CALLS]->(b) WHERE ..."` |
| Natural-language search | `rgctl_search` | `semantic query "checkout flow"` |
| List communities | `rgctl_query` macro: `all_communities` | `communities list` |
| Community members | `rgctl_query` | `gql "MATCH (f) WHERE f.community_id='12'"` |
| Subsystem ownership | `rgctl_search` scope: `community` | `semantic query "X" --scope community` |
| Refresh community labels | — (CLI only) | `communities label --write` |

**Community detection:**
- Reveals implicit architecture (functional clusters)
- Use for microservice extraction, ownership mapping, coupling analysis
- Communities span package boundaries based on call patterns

**GQL limitations:**
- No `COUNT`, `ORDER BY`, `GROUP BY`
- LIKE only works for prefix (`Foo*`) or suffix (`*Foo`) — `*middle*` silently returns 0
- CALLS edges miss dynamic dispatch — fall back to `grep` if 0 results for known calls

**See:** [GQL Reference](references/gql-reference.md), [Communities & Policy Guide](references/communities-and-policy.md), [Semantic Search Guide](../../docs/guides/semantic-search.md)

### 3. Impact & Safety

**When:** Pre-refactor, CI gates, architectural policy

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| Blast radius | `rgctl_impact` | `blast-radius <Symbol> --depth N` |
| Policy check (full codebase) | `rgctl_check` | `check --policy-file policy.json` |
| Policy check (one symbol) | `rgctl_impact` | `blast-radius <Symbol> --policy-file policy.json` |

**Blast-radius tips:**
- Add `--class` or `--file` if symbol is ambiguous
- Returns score, direct callers, impact zone size
- Interface/dynamic dispatch may return score=0 — fall back to `grep`

**Policy rules:**
- `max_impact_nodes` — Max blast-radius impact zone
- `centrality_alert_threshold` — PageRank centrality limit
- `forbidden_crossings` — Disallowed call patterns (e.g., controller → database)
- Exit code 1 on violations (suitable for CI gates)

**See:** [Communities & Policy Guide](references/communities-and-policy.md), [Blast Radius Guide](../../docs/guides/blast-radius-analysis.md), [CI Policy Guide](../../docs/guides/ci-policy-checks.md)

### 4. Metrics & Analysis

**When:** Find hotspots, bottlenecks, central dependencies

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| PageRank hotspots | `rgctl_metrics` | `metrics --pagerank` |
| Betweenness bridges | `rgctl_metrics` | `metrics --betweenness` |
| Community stats | `rgctl_metrics` | `metrics --communities` |

**PageRank note:** Returns UUIDs, not names. Resolve with `cpg function <uuid>` or `blast-radius <uuid>`.

**Use metrics with policy:**
1. Run `metrics --pagerank` to find current hotspots
2. Set `max_impact_nodes` to 90th percentile
3. Use `centrality_alert_threshold` to flag high-centrality functions
4. Gradually tighten thresholds as architecture improves

**See:** [Graph Metrics Guide](../../docs/guides/graph-metrics.md), [Communities & Policy Guide](references/communities-and-policy.md)

### 5. Code Analysis (CFG/PDG/Slicing)

**When:** Data flow, mutations, loop hazards, taint analysis

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| CFG/PDG status | `rgctl_cpg` op: `status` | `cpg status` |
| Field mutations | `rgctl_cpg` op: `mutations` | `cpg mutations --type Foo` |
| Data flow trace | `rgctl_cpg` op: `flows` | `cpg flows FILE --line N --variable V --function F` |
| Program slice | `rgctl_cpg` op: `slice` | `slice FILE --line N --variable V --function F` |
| CFG inspection | `rgctl_cpg` op: `inspect` | `inspect <Symbol> cfg` |
| PDG inspection | `rgctl_cpg` op: `inspect` | `inspect <Symbol> pdg --edge-layer data` |

**Slice/flow tips:**
- `--function` is **method name**, not class
- `--line` must be inside function body (not struct/import)
- Needs `discover --with-cfg`

**See:** [Hybrid CPG Guide](../../docs/guides/hybrid-cpg.md), [Slicing Guide](../../docs/guides/program-slicing.md)

### 6. Export & Visualization

**When:** Generate diagrams, serve dashboard, export for external tools

| User Intent | MCP Tool | CLI Command |
|-------------|----------|-------------|
| Export graph | — | `export --export-format graphviz --export-output OUT` |
| CPG export | — | `cpg export --format graphson --output cpg.json` |
| HTTP dashboard | — | `serve --open` |

**Export format tips:**
- `--query` uses **filter syntax** (`name:Foo`, `type:Function`, `all`) — **not** GQL MATCH
- Dashboard requires `discover --with-dashboard`

**See:** [Exporting Graphs Guide](../../docs/guides/exporting-graphs.md), [HTTP Server Guide](../../docs/guides/http-server-and-dashboard.md)

## Natural Language → Tool Routing

Quick decision table for common user utterances:

| User Says | Tool/Command |
|-----------|--------------|
| "Generate migration plan / modernize" | `discover --with-cfg --with-security --export-migration-hints --migration-preset hybrid_default` then read `.rgctl/migration_plan.json` |
| "Bottlenecks / hotspots / central dependencies" | `metrics --pagerank` |
| "Inventory functions / candidates to delete" | `gql --macro-name all_functions unused` |
| "What communities exist?" | `communities list` |
| "Show implicit architecture" | `communities list` + explore largest communities |
| "Which subsystem owns X?" | `semantic query "X" --scope community` |
| "List functions in community N" | `gql "MATCH (f) WHERE f.community_id='N' RETURN f"` |
| "Find coupling between modules" | `gql` with cross-community CALLS patterns |
| "Where is checkout flow?" | `semantic query "checkout flow" --limit 10` |
| "Find all *Service" | `gql "MATCH (n:Function) WHERE n.name LIKE '*Service' RETURN n"` |
| "Impact if I change X" | `blast-radius X --depth 2` |
| "Validate against policy" | `check --policy-file policy.json` |
| "Set up CI gate / enforce rules" | Create policy.json → `check --policy-file policy.json` in CI |
| "What violates our policy?" | `check --policy-file policy.json` (parse violations array) |
| "Is this change safe to merge?" | `blast-radius X --policy-file policy.json` |
| "Call stack / who calls X" | `gql "MATCH (a)-[:CALLS*1..3]->(b) WHERE a.name='X' RETURN a,b"` |
| "Where is X mutated?" | `cpg mutations --type X --exclude-ctors` |
| "Trace variable flow" | `cpg flows FILE --line N --variable V --function F` |
| "Loop hazards / parallelization" | `discover --with-dfg-loops` then `inspect <Symbol> pdg --edge-layer data` (look for `loop_carried`) |
| "Relationship between X and Y" | `gql` CALLS/DEPENDSON path or `blast-radius` on each |

## Common Scenarios

### Migration Planning

```bash
# Heavy discover with all migration features
rgctl discover . --with-cfg --with-security --with-taint \
  --with-dashboard --with-harmonic --export-migration-hints \
  --migration-preset foundational_first --migration-order scheduled

# Read the plan (NOT in discover stdout)
cat .rgctl/migration_plan.json
```

**Presets:** `hybrid_default`, `foundational_first`, `dense_cluster`, `risk_mitigation`  
**Orders:** `scheduled` (dependency-safe), `priority` (highest-impact first)

**See:** [Workflows Reference](references/workflows.md#migration--audit)

### Pre-Refactor Safety Check

Before refactoring `updateQuantity`:

```bash
# 1. Impact analysis
rgctl -f json blast-radius updateQuantity --depth 2

# 2. Call neighborhood
rgctl -f json gql "MATCH (a)-[:CALLS*1..3]->(b) WHERE a.name='updateQuantity' RETURN a,b"

# 3. Data flow (needs --with-cfg)
rgctl -f json cpg flows src/service.ts --line 50 --variable quantity --function updateQuantity --direction forward
```

**See:** [Workflows Reference](references/workflows.md#pre-refactor-safety-analysis)

### Concept Search (When LIKE Returns 0)

If GQL LIKE returns 0 for a concept like "gateway":

1. Try `communities list` and grep labels
2. Try `semantic query "gateway"`
3. Broaden LIKE to non-Function types: `MATCH (n) WHERE n.name LIKE '*Gateway*'`

Concepts often live in package/directory names or type names, not function names.

## Failure Playbook

| Symptom | Fix |
|---------|-----|
| No `.rgctl/` in repo | Normal with default daemon — check `~/.rgctl/cache/` or run `cd repo && rgctl discover .`; use `--no-daemon` for in-repo artifacts |
| slice/inspect/cpg fails | Re-discover with `--with-cfg` |
| semantic query fails | `semantic index` |
| Ambiguous symbol | Add `--class` or `--file`; disambiguate via GQL |
| `check` exit 1 | Report violations (JSON still on stdout) |
| blast-radius returns 0 for known method | Interface/dynamic dispatch — fall back to `grep` |
| GQL LIKE returns 0 | Concept in package/directory path — try `communities list`, `semantic query`, or broader LIKE |
| `export --query` with MATCH fails | Use filter syntax (`name:Foo`, `type:Function`, `all`) |

## Artifacts

Paths are under the artifact root (`~/.rgctl/cache/{reponame}/.rgctl/` by default, or `{repo}/.rgctl/` with `--no-daemon`):

| Path | Content |
|------|---------|
| `graph.snapshot.bin` | Main graph snapshot |
| `semantic_index.bin` | Semantic index |
| `migration_plan.json` | Migration roadmap |
| `dashboard/` | Dashboard bundle |
| `analysis/` | CFG/PDG archives |

## Exit Codes

- `0` — Success
- `1` — Policy violation (`check`) or command error

## Usage Globals

```bash
# Typical workflow
export REPO=/path/to/repo
rgctl -r "$REPO" discover
rgctl -r "$REPO" -f json <command> …
```

**Globals:** `-f json` (agents), `-r` / `--repo`, `-o` output file, `-d` / `--db` (custom graph cache path)

## Reference Files

For detailed command syntax, JSON schemas, and advanced patterns:

- **[Command Encyclopedia](references/command-encyclopedia.md)** — Full command reference with samples
- **[Workflows](references/workflows.md)** — Worked scenarios (migration, refactor, audit)
- **[GQL Reference](references/gql-reference.md)** — GQL patterns and limitations
- **[Communities & Policy](references/communities-and-policy.md)** — Community detection, CI policy checks, architectural rules

## External Documentation

- [User Guide](../../docs/user-guide.md) — Complete CLI tutorial
- [JSON API](../../docs/json-api.md) — Schema specifications
- [Agent Recipes](../../docs/agent-recipes.md) — Copy-paste recipes
- [MCP Server Guide](../../docs/guides/mcp-server.md) — MCP setup and tools
- [Policy Format](../../docs/policy-format.md) — CI policy schema

## Installation

From another repo or OpenCode:

```bash
rgctl install --skill
```

This writes `.claude/skills/rgctl/` and `.cursor/skills/rgctl/` from the embedded skill in the binary.
