# HTTP Server and Dashboard

## Introduction

The `serve` command launches an HTTP server that provides both a **browser-based dashboard** for visual exploration and a **query API** for programmatic access. The dashboard lets you explore the code graph, run GQL queries, visualize blast radius, inspect CFGs, and browse communities -- all from your browser. The API endpoints (`/api/query`, `/api/semantic/*`) let agents and scripts issue queries without spawning a new process for each one.

## Use Cases

- **Interactive exploration.** Browse functions, classes, and communities in a visual UI.
- **Persistent query session.** Keep the server running and issue repeated queries over HTTP instead of spawning a new CLI process each time.
- **Team sharing.** Run the server on a shared host so the whole team can explore the codebase.
- **Agent integration.** LLM agents can POST queries to `/api/query` for low-latency, stateful graph access.
- **Demo and presentation.** Show stakeholders the architecture of a codebase through the dashboard.

## Example Project

This guide uses the **CoolStore** (`example/coolstore`). `rgctl serve` can start the full pipeline itself. To only serve existing artifacts:

```bash
rgctl -r example/coolstore discover . --with-cfg --with-dashboard
rgctl -r example/coolstore serve --no-pipeline --open
```

The `--with-dashboard` flag exports the static dashboard bundle to `.rgbuilder/dashboard/`.

## Step-by-Step

### 1. Start the Server

Launch the HTTP server with the dashboard:

```bash
rgctl -r example/coolstore serve --open
```

**What happens:**

- The server starts on `http://127.0.0.1:8080`.
- The `--open` flag opens the dashboard in your default browser.
- The dashboard serves from `.rgbuilder/dashboard/` and the query API is available at `/api/query`.

### 2. Custom Host and Port

Bind to a different address or port:

```bash
rgctl -r example/coolstore serve --host 0.0.0.0 --port 3000
```

This makes the server accessible on all network interfaces at port 3000, useful for team sharing.

### 3. Query API Only

If you only need the API (no dashboard UI):

```bash
rgctl -r example/coolstore serve --query-only
```

This starts a lighter server with only the `/api/query` and `/api/semantic/*` endpoints.

### 4. Dashboard Only

If you only need the visual dashboard:

```bash
rgctl -r example/coolstore serve --dashboard-only
```

### 5. Querying the API with curl

With the server running, issue GQL queries via HTTP:

```bash
curl -s http://127.0.0.1:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (f:Function) WHERE f.name = '\''priceShoppingCart'\'' RETURN f"}'
```

**Response:**

```json
{
  "count": 1,
  "rows": [
    [
      {
        "binding": "f",
        "file": "example/coolstore/./src/main/java/com/redhat/coolstore/service/ShoppingCartService.java",
        "node": "priceShoppingCart",
        "qualified_name": "com.redhat.coolstore.service.ShoppingCartService.priceShoppingCart",
        "type": "Function"
      }
    ]
  ],
  "schema_version": 1
}
```

### 6. Semantic Search via API

Query the semantic index over HTTP:

```bash
curl -s http://127.0.0.1:8080/api/semantic/query \
  -H "Content-Type: application/json" \
  -d '{"query": "shopping cart checkout", "limit": 5}'
```

### 7. Background daemon (HTTP + MCP)

Start a shared background daemon (catalog, per-repo HTTP, MCP at `/mcp`). Default cache: `~/.rgbuilder/cache/{reponame}/` (override with `--daemon-home` / `RGCTL_HOME`).

```bash
rgctl daemon start --host 127.0.0.1 --port 8080
# or foreground bootstrap:
rgctl -r example/coolstore serve --daemon
```

Cached repos are served under `http://127.0.0.1:8080/{reponame}/…`. Foreground `rgctl serve` (without `--daemon`) stays `127.0.0.1:8080` with unprefixed `/api/*` for one repo.

### 8. Daemon idle timeout

`--idle-secs` applies to **`serve --daemon`** only (default 300s). The HTTP server does not auto-exit on idle.

```bash
rgctl -r example/coolstore serve --daemon --idle-secs 3600
```

## Dashboard Tabs

When the dashboard opens in your browser, you will see several tabs:

| Tab | Description |
|-----|-------------|
| **Search** | Full-text and semantic search across functions and classes |
| **Graph** | Interactive force-directed graph visualization |
| **Functions** | Sortable table of all functions with metrics |
| **CFG** | Control-flow graph viewer for individual functions |
| **Dataflow** | Data-flow and PDG visualization |
| **Slice** | Interactive program slicing |
| **Blast** | Blast radius visualization with caller/impact trees |
| **Taint** | Taint analysis results (requires `--with-taint`) |
| **Migration** | Migration roadmap viewer (requires `--export-migration-hints`) |
| **Query Guide** | Built-in GQL query reference |

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/query` | POST | Execute a GQL query (HTTP 503 + pipeline status if the graph is not ready) |
| `/api/status` | GET | Full-pipeline status (`schema_version` 1) |
| `/api/semantic/query` | POST | Semantic search |
| `/api/semantic/index` | POST | Trigger semantic indexing |
| `/` | GET | Dashboard UI |

See the [HTTP API Reference](../http-api.md) for complete endpoint documentation.

## Server Options Reference

| Option | Default | Description |
|--------|---------|-------------|
| `--host` | `127.0.0.1` | Bind host |
| `--port` | `8080` | HTTP port |
| `--open` | off | Open dashboard in browser (preparing page if the bundle is not ready) |
| `--query-only` | off | Serve API only, no dashboard |
| `--dashboard-only` | off | Serve dashboard only, no API |
| `--mode` | `standard` | `standard` (HTTP) or `mcp` (stdio, no HTTP) |
| `--no-pipeline` | off | Fail fast if artifacts are missing (old `serve` behavior) |
| `--dashboard-dir` | `.rgbuilder/dashboard` | Dashboard directory |
| `--daemon` | off | Legacy Unix socket mode |
| `--daemon` | off | Background HTTP+MCP daemon (`daemon start`) |
| `--idle-secs` | `300` | Auto-exit after N seconds idle |

## Benefits

- **Zero setup.** One command to launch a full-featured analysis dashboard.
- **Dual interface.** Visual dashboard for humans, HTTP API for agents and scripts.
- **Session persistence.** The server keeps the graph in memory, making repeated queries fast.
- **Team accessible.** Bind to `0.0.0.0` to share the dashboard across a network.
- **Low resource.** HTTP `serve` stays up until Ctrl+C. `--daemon` idle-exits (default 300s).

## Related Guides

- [Discovering and Indexing a Codebase](discovering-and-indexing.md) -- `discover --with-dashboard` generates the dashboard bundle
- [MCP Server](mcp-server.md) -- `serve --mode mcp` for Cursor / Claude Code (no HTTP)
- [Graph Query Language](graph-query-language.md) -- the query language used by the API
- [Semantic Search](semantic-search.md) -- semantic queries available via `/api/semantic/*`
- [Blast Radius Analysis](blast-radius-analysis.md) -- blast-radius visualization in the dashboard
