# MCP Server

## Introduction

`rg-build serve --mode mcp` starts an **MCP** (Model Context Protocol) server on **stdio**. Cursor, Claude Code, and other MCP hosts spawn it as a subprocess and talk JSON-RPC on stdin/stdout. The process **does not bind HTTP** -- that stays `rg-build serve` (standard mode).

On start it detects the session root (`-r` / `--repo`, an optional PATH, or the current working directory) and, unless you pass `--no-pipeline`, begins the same **full pipeline** as `discover --full`: basic graph, then CFG + dashboard + harmonic, then semantic index.

The catalog is **seven tools** (workflow families, not one CLI command per tool). `discover`, `semantic index`, `export`, `communities label --write`, and the dashboard are **not** MCP tools -- the pipeline already runs on serve, and those stay CLI. When a tool needs an artifact that is not ready, it returns **pipeline status JSON** as the tool result (not a JSON-RPC error).

CLI `-f json gql` does **not** apply a default row cap. MCP `rgbuilder_query` and `rgbuilder_search` default `limit` to **20** when the client omits it.

## Use Cases

- **IDE session.** Leave MCP connected in Cursor while you edit CoolStore; the graph warms in the background.
- **Structural questions in the editor.** Query, semantic search, blast-radius, metrics, CPG, and policy check without spawning a new `rg-build` per turn.
- **Pipeline readiness.** `rgbuilder_status` (and unreadiness results from other tools) report dashboard, CFG archive, and semantic index flags.
- **No stdout scraping.** Progress and logs go to **stderr**. stdout is JSON-RPC only, so the host never has to parse CLI banners.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). You can start MCP with an empty `.rgbuilder/` -- the server will index for you.

```bash
rg-build -r example/coolstore serve --mode mcp
```

The host should set the working directory to the repo (or pass `-r`). Do not run this in a terminal you want to type into: it waits on stdin.

## Step-by-Step

### 1. Understand standard vs MCP

| Command | Transport | Dashboard | Auto full pipeline |
|---------|-----------|-----------|--------------------|
| `rg-build serve` | HTTP `127.0.0.1:8080` | Yes (preparing page until ready) | Yes (unless `--no-pipeline`) |
| `rg-build serve --mode mcp` | stdio JSON-RPC | Bundle on disk only -- no HTTP | Yes (unless `--no-pipeline`) |
| `rg-build serve --daemon` | Unix socket | No | No |

Use **MCP in the IDE** for the seven tools. Use **CLI** for CI, `discover`, `semantic index`, `cpg export`, and one-shot scripts.

### 2. Configure Cursor

Project `.cursor/mcp.json` (or your user MCP config). Use an **absolute** `-r` path: GUI apps often do not inherit your shell cwd.

```json
{
  "mcpServers": {
    "rgbuilder": {
      "command": "rg-build",
      "args": ["-r", "/absolute/path/to/rgBuilder/example/coolstore", "serve", "--mode", "mcp"]
    }
  }
}
```

If the host already starts the process in the CoolStore directory, you can omit `-r`:

```json
{
  "mcpServers": {
    "rgbuilder": {
      "command": "rg-build",
      "args": ["serve", "--mode", "mcp"]
    }
  }
}
```

Restart MCP (or reload the window) so Cursor spawns the server. Confirm `rg-build` is on `PATH` for the GUI app (not only your shell).

### 3. Configure Claude Code

Project `.mcp.json`:

```json
{
  "mcpServers": {
    "rgbuilder": {
      "command": "rg-build",
      "args": ["serve", "--mode", "mcp"]
    }
  }
}
```

Run Claude Code from the repository root so cwd is the session root.

### 4. What happens on initialize

1. The host sends MCP `initialize`.
2. rgBuilder advertises the seven tools, three resources, and server name `rgbuilder`.
3. A background thread starts **basic discover**, then CFG + dashboard + harmonic, then `semantic index` (vocab embedder).
4. After stage 1, `.rgbuilder/graph.snapshot.bin` is on disk -- CLI `gql` / `blast-radius` work from another terminal while MCP stays up.
5. Call `rgbuilder_status` (or any tool that needs artifacts) until `dashboard_ready`, `cfg_ready`, and `semantic_ready` are true for deep / semantic work.

Logs such as `[!] full pipeline: ...` go to **stderr**. Never write non-RPC bytes to stdout in this mode.

### 5. Call `rgbuilder_status`

The tool takes no arguments. The result includes pretty-printed JSON text and `structuredContent` with the same fields as `GET /api/status` / `.rgbuilder/pipeline_status.json`.

| Field | Meaning |
|-------|---------|
| `schema_version` | `1` |
| `command` | `pipeline_status` |
| `plan[]` | Stages `basic_discover`, `deep_pass`, `semantic_index` with `pending` / `running` / `complete` / `skipped` / `failed` |
| `dashboard_ready` | `.rgbuilder/dashboard/index.html` exists |
| `cfg_ready` | CFG/PDG archive exists |
| `semantic_ready` | `semantic_index.bin` exists |
| `message` | Human line, e.g. `Dashboard is being prepared` |

Example `structuredContent` while the dashboard is still building:

```json
{
  "schema_version": 1,
  "command": "pipeline_status",
  "repo": "/path/to/example/coolstore",
  "mode": "full",
  "phase": "deep_pass",
  "dashboard_ready": false,
  "cfg_ready": false,
  "semantic_ready": false,
  "message": "Dashboard is being prepared",
  "plan": [
    { "id": "basic_discover", "status": "complete" },
    { "id": "deep_pass", "status": "running" },
    { "id": "semantic_index", "status": "pending" }
  ]
}
```

When `dashboard_ready` is true, open the UI with a **separate** HTTP server if you want a browser:

```bash
rg-build -r example/coolstore serve --no-pipeline --open
```

`--no-pipeline` here skips a second full pipeline if artifacts are already in place.

### 6. Query, search, impact, metrics, CPG, check

JSON shapes match CLI `-f json` for the same operation ([JSON API](../json-api.md)). Pass either `query` or `macro` on `rgbuilder_query`, not both.

| Tool | Typical arguments | Notes |
|------|-------------------|--------|
| `rgbuilder_query` | `query` **or** `macro`; optional `explain`, `limit` | Macros: `all_functions`, `all_communities`, `direct_calls`, `call_chain`. Default `limit` 20. Find-by-name / FQN / community / neighborhood are GQL, not extra tools. |
| `rgbuilder_search` | `text`; optional `scope`, `limit` | `scope`: `function` (default), `community`, `docs`, `all`. Default `limit` 20. Needs semantic index. |
| `rgbuilder_impact` | `symbol`; optional `depth`, `class`, `file` | Blast-radius JSON (`schema_version` 2) including `metrics`. |
| `rgbuilder_metrics` | at least one of `pagerank`, `betweenness`, `communities` | Invalid-params if no flag. |
| `rgbuilder_cpg` | `op` plus op-specific fields | `op`: `status`, `function`, `calls`, `mutations`, `flows`, `slice`, `inspect`, `pdg`, `ast`. **Not** `export` (CLI only). |
| `rgbuilder_check` | `policy_file` | Same as CLI `-f json check`. |

Cursor / Claude examples:

- Inventory: `rgbuilder_query` with `{ "macro": "all_functions" }`
- Name search: `rgbuilder_query` with `{ "query": "MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n" }`
- Checkout: `rgbuilder_search` with `{ "text": "checkout flow", "scope": "function" }`
- Before edit: `rgbuilder_impact` with `{ "symbol": "CartService::clearCart", "depth": 2 }`
- Hotspots: `rgbuilder_metrics` with `{ "pagerank": true }`
- CPG: `rgbuilder_cpg` with `{ "op": "status" }` then `{ "op": "mutations", "type_name": "ShoppingCart", "exclude_ctors": true }`
- CI: `rgbuilder_check` with `{ "policy_file": "/absolute/path/to/policy.json" }`

### 7. Resources

`resources/list` advertises:

| URI | Content |
|-----|---------|
| `rgbuilder://status` | Pipeline status (`schema_version` 1) |
| `rgbuilder://manifest` | Dashboard `manifest.json`, or status JSON if missing |
| `rgbuilder://migration-plan` | Migration plan JSON, or `{ "available": false, ... }` if missing |

### 8. Smoke-test without an IDE

From the CoolStore directory (another terminal can run `gql` after stage 1):

```bash
rg-build -r example/coolstore serve --mode mcp
```

In a second process you would normally let the IDE speak MCP. For a raw check, send newline-delimited JSON-RPC (the server also accepts `Content-Length` framing):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"rgbuilder_status","arguments":{}}}
```

Responses are framed with `Content-Length`. A successful `tools/call` includes `result.structuredContent`.

### 9. Skip auto-indexing

If the graph is already built:

```bash
rg-build -r example/coolstore serve --mode mcp --no-pipeline
```

Without a snapshot, discover will not run; tools that need the graph return pipeline status for whatever artifacts are on disk.

### 10. Session root

Same rules as `discover` / HTTP `serve`:

- `-r` / `--repo` if set
- else positional `PATH` on `serve`
- else process cwd (what Cursor sets for the MCP child)

Two full pipelines on the same repo cannot run at once (`.rgbuilder/pipeline.lock`).

## Tools

| MCP tool | Arguments | Notes |
|----------|-----------|--------|
| `rgbuilder_status` | none | Pipeline + artifact flags. Safe at any time. |
| `rgbuilder_query` | `query` xor `macro`; `explain`; `limit` | Default limit 20. |
| `rgbuilder_search` | `text`; `scope`; `limit` | Default limit 20. Unready → status JSON. |
| `rgbuilder_impact` | `symbol`; `depth`; `class`; `file` | Blast-radius. |
| `rgbuilder_metrics` | `pagerank` / `betweenness` / `communities` | At least one required. |
| `rgbuilder_cpg` | `op` + fields | No `export`. CFG ops unready → status JSON. |
| `rgbuilder_check` | `policy_file` | Missing file is invalid-params. |

**Not MCP tools** (CLI): `discover`, `semantic index`, `cpg export`, `communities label --write`, dashboard HTTP.

Also install the agent skill so the host knows CLI recipes for those:

```bash
rg-build -r example/coolstore install --skill
```

When MCP is already connected in the IDE, prefer the seven tools over spawning `rg-build -f json` for the same intents.

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--mode mcp` | (off; default mode is `standard`) | stdio MCP, no HTTP bind |
| `--no-pipeline` | off | Do not start `discover --full` |
| `-r` / `PATH` | cwd | Session root |
| `--daemon` | -- | **Cannot** combine with `--mode mcp` |

## Benefits

- **One long-lived session** in the editor instead of a cold `rg-build` per question.
- **Honest unreadiness** -- status JSON as a tool result when the graph, CFG, or semantic index is still building.
- **Safe stdio** -- RPC on stdout, logs on stderr.
- **Same pipeline as `--full`** -- CFG, dashboard bundle, harmonic, and semantic index without extra flags.
- **Same JSON as CLI** -- `schema_version` fields from the shared command service.

## Related Guides

- [HTTP Server and Dashboard](http-server-and-dashboard.md) -- browser UI and `/api/query` (standard `serve`)
- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover --full` stages
- [Agent Skill](agent-skill.md) -- `install --skill` for Claude Code and Cursor
- [Graph Query Language](graph-query-language.md) -- MATCH / macros (`rgbuilder_query`)
- [JSON API](../json-api.md) -- payload shapes shared by CLI, HTTP, and MCP
