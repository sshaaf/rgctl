# HTTP query API (`rgctl serve`)

`rgctl serve` starts a local HTTP server that serves the **static dashboard** and a **GQL query API** on the same origin.

**CLI reference:** [User Guide §15](user-guide.md#15-http-server-serve)

Default `rgctl serve` **starts the full discover pipeline** (unless `--no-pipeline`) and binds HTTP even if the dashboard bundle is not ready yet. `GET /` returns a preparing page until `index.html` exists. `GET /api/status` is the pipeline document (`schema_version` 1). `--mode mcp` speaks MCP on stdio and does **not** bind HTTP (seven workflow tools). Walkthrough: [MCP Server](guides/mcp-server.md). `--daemon` does not auto-discover.

---

## Default behavior

```bash
rgctl -r "$REPO" serve
# or: rgctl discover . --full && rgctl serve --no-pipeline
```

| URL | Purpose |
|-----|---------|
| `http://127.0.0.1:8080/` | Dashboard (`index.html`) |
| `http://127.0.0.1:8080/api/query` | GQL / macro queries (POST JSON) |
| `http://127.0.0.1:8080/graphql` | Alias for `/api/query` |
| `http://127.0.0.1:8080/api/health` | Health check (GET) |
| `http://127.0.0.1:8080/api/status` | Full-pipeline status (GET) |
| `http://127.0.0.1:8080/api/semantic/status` | Semantic index status (GET) |
| `http://127.0.0.1:8080/api/semantic/query` | Semantic search (POST JSON) |

Open browser automatically:

```bash
rgctl -r "$REPO" serve --open
```

### Options

| Flag | Effect |
|------|--------|
| `--host`, `--port` | Bind address (default `127.0.0.1:8080`) |
| `--dashboard-dir DIR` | Override `.rgbuilder/dashboard` |
| `--query-only` | API only, no static files |
| `--dashboard-only` | Dashboard only, no query API |
| `--mode standard\|mcp` | HTTP (default) or MCP stdio |
| `--no-pipeline` | Do not auto-discover; fail if dashboard/graph missing |
| `--daemon` | **Legacy** Unix-socket blast daemon (no HTTP, no pipeline) |

---

## Query API

### Request

`POST /api/query` with `Content-Type: application/json`

**GQL query:**

```json
{
  "query": "MATCH (n:Function) WHERE n.name LIKE '*Service*' RETURN n LIMIT 10"
}
```

**Macro:**

```json
{
  "macro": "all_functions"
}
```

**Explain plan:**

```json
{
  "query": "MATCH (n:Function) RETURN n LIMIT 5",
  "explain": true
}
```

### curl example

```bash
curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"macro":"all_functions"}' | jq '.count'

curl -sS -X POST http://127.0.0.1:8080/api/query \
  -H 'Content-Type: application/json' \
  -d '{"macro":"all_communities"}' | jq '.rows[:5]'
```

`serve` loads `.rgbuilder/analysis_results.bin` so virtual `:Community` nodes and `community_id` filters work the same as CLI `gql`.

### Response

Same JSON shape as `rgctl -f json gql` on the CLI. See [json-api.md](json-api.md) §5.

Errors return HTTP 400 with a plain-text message body.

---

## Semantic search API

Requires `rgctl semantic index` before `serve` (embedder chosen at index time: **vocab** default, or `hash` / `code-daemon` / `onnx`). Restart `serve` after rebuilding `.rgbuilder/semantic_index.bin`. Same origin as the dashboard.

### `GET /api/semantic/status`

Returns JSON: `{ "available": true, "model_id": "...", "dimensions": N, "functions_indexed": N }` when the index loaded (`model_id` may be `code-daemon:v1`, `vocab-accumulate-v1`, `sign-hash-v1`, …).

### `POST /api/semantic/query`

`Content-Type: application/json`

```json
{
  "query": "shopping cart checkout",
  "limit": 20,
  "fusion": true,
  "keyword_and": false,
  "scope": "function"
}
```

`scope` may be `"function"` (default) or `"community"` (pooled member embeddings; requires discover analysis).

Response matches `rgctl -f json semantic query`. Errors return HTTP 503 when the index is missing.

```bash
curl -sS http://127.0.0.1:8080/api/semantic/status | jq .
curl -sS -X POST http://127.0.0.1:8080/api/semantic/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"OrderService","limit":5}' | jq '.hits[:3]'
curl -sS -X POST http://127.0.0.1:8080/api/semantic/query \
  -H 'Content-Type: application/json' \
  -d '{"query":"checkout","scope":"community","limit":5}' | jq '.hits'
```

---

## Serving dashboard without the API

Static hosting (no Rust process after export):

```bash
cd .rgbuilder/dashboard && python3 -m http.server 8765
# open http://localhost:8765/
```

WASM requires HTTP (not `file://`). The in-browser worker cannot run full GQL — use `rgctl serve` for live queries or the CLI.

---

## Background HTTP+MCP daemon

Shared daemon for multi-repo cache, catalog, and MCP:

```bash
rgctl daemon start [--host HOST] [--port PORT]
rgctl -r "$REPO" discover .
rgctl -r "$REPO" serve --daemon   # foreground bootstrap; same daemon model
```

- **Catalog:** `GET http://127.0.0.1:8080/`
- **Per-repo API:** `POST http://127.0.0.1:8080/{reponame}/api/query`
- **MCP:** `POST http://127.0.0.1:8080/mcp`

CLI commands route through the daemon by default (cache under `~/.rgbuilder/`). Use **`--no-daemon`** for in-process execution and `{repo}/.rgbuilder/` artifacts.

The legacy blast-radius-only **`query.sock`** auto-connect path is **retired** on current main (see [unreleased](releases/unreleased.md)).

---

## Not exposed over HTTP

These CLI surfaces are **not** available as HTTP routes today (use `-f json` on the CLI instead):

- `blast-radius`, `metrics`, `check`, `slice`, `inspect`
- `communities`, `cpg`, `export`
- `discover` (indexing remains a local CLI operation)

**Exposed today:** `POST /api/query` (GQL), `GET/POST /api/semantic/*` (see above), plus the static dashboard UI.

---

## See also

- [AGENTS.md](../AGENTS.md) — agent integration patterns
- [MCP Server](guides/mcp-server.md) — `serve --mode mcp` (stdio, no HTTP)
- [Dashboard user guide](dashboard-user-guide.md) — browser UI
