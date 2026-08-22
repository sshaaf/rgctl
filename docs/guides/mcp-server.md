# MCP Server

## Introduction

`rg-build serve --mode mcp` starts an **MCP** (Model Context Protocol) server on **stdio**. Cursor, Claude Code, and other MCP hosts spawn it as a subprocess and talk JSON-RPC on stdin/stdout. The process **does not bind HTTP** -- that stays `rg-build serve` (standard mode).

On start it detects the session root (`-r` / `--repo`, an optional PATH, or the current working directory) and, unless you pass `--no-pipeline`, begins the same **full pipeline** as `discover --full`: basic graph, then CFG + dashboard + harmonic, then semantic index. Tools that need artifacts return a **status** until those files exist.

Today the MCP surface is **status-only** (`rgbuilder_status`). Graph queries still go through the CLI (`-f json`) or HTTP `serve`. A fuller tool catalog is tracked in [issue #60](https://github.com/sshaaf/rgBuilder/issues/60).

## Use Cases

- **IDE session.** Leave MCP connected in Cursor while you edit CoolStore; the graph warms in the background.
- **Pipeline readiness.** Ask whether the dashboard, CFG archive, or semantic index is ready before opening the UI or running semantic search.
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
2. rgBuilder advertises tools (`rgbuilder_status`) and server name `rgbuilder`.
3. A background thread starts **basic discover**, then CFG + dashboard + harmonic, then `semantic index` (vocab embedder).
4. After stage 1, `.rgbuilder/graph.snapshot.bin` is on disk -- CLI `gql` / `blast-radius` work from another terminal while MCP stays up.
5. Call `rgbuilder_status` until `dashboard_ready`, `cfg_ready`, and `semantic_ready` are true.

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

### 6. Smoke-test without an IDE

From the CoolStore directory (another terminal can run `gql` after stage 1):

```bash
rg-build -r example/coolstore serve --mode mcp
```

In a second process you would normally let the IDE speak MCP. For a raw check, send newline-delimited JSON-RPC (the server also accepts `Content-Length` framing):

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}
```

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"rgbuilder_status","arguments":{}}}
```

Responses are framed with `Content-Length`. A successful `tools/call` includes `result.structuredContent.schema_version` `1`.

### 7. Skip auto-indexing

If the graph is already built and you only want status:

```bash
rg-build -r example/coolstore serve --mode mcp --no-pipeline
```

Without a snapshot, discover will not run; `rgbuilder_status` still reports whatever artifacts are on disk.

### 8. Session root

Same rules as `discover` / HTTP `serve`:

- `-r` / `--repo` if set
- else positional `PATH` on `serve`
- else process cwd (what Cursor sets for the MCP child)

Two full pipelines on the same repo cannot run at once (`.rgbuilder/pipeline.lock`).

## Tools (current)

| MCP tool | Arguments | Notes |
|----------|-----------|--------|
| `rgbuilder_status` | none | Pipeline + artifact flags. Safe to call at any time. |

**Not yet MCP tools** (use CLI `-f json` or HTTP `/api/query`): `gql`, `blast-radius`, `semantic query`, `slice`, `cpg`, `check`. See [issue #60](https://github.com/sshaaf/rgBuilder/issues/60).

Also install the agent skill so the host knows the CLI recipes:

```bash
rg-build -r example/coolstore install --skill
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--mode mcp` | (off; default mode is `standard`) | stdio MCP, no HTTP bind |
| `--no-pipeline` | off | Do not start `discover --full` |
| `-r` / `PATH` | cwd | Session root |
| `--daemon` | -- | **Cannot** combine with `--mode mcp` |

## Benefits

- **One long-lived session** in the editor instead of a cold `rg-build` per question.
- **Honest unreadiness** -- status instead of a crash when the dashboard or semantic index is still building.
- **Safe stdio** -- RPC on stdout, logs on stderr.
- **Same pipeline as `--full`** -- CFG, dashboard bundle, harmonic, and semantic index without extra flags.

## Related Guides

- [HTTP Server and Dashboard](http-server-and-dashboard.md) -- browser UI and `/api/query` (standard `serve`)
- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover --full` stages
- [Agent Skill](agent-skill.md) -- `install --skill` for Claude Code and Cursor
- [Graph Query Language](graph-query-language.md) -- query the graph from the CLI until MCP grows a `gql` tool
- [JSON API](../json-api.md) -- `pipeline_status` and discover `--full` payloads
