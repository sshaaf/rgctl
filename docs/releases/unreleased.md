# Unreleased (post v0.4.8)

## Breaking changes

### Daemon mode and MCP removed

Background daemon mode, stdio MCP (`serve --mode mcp`), and the `daemon` subcommand are **removed**. All commands run in-process against **in-repo** artifacts at `{repo}/.rgctl/`.

**Removed flags / commands:**

- Global: `--no-daemon`, `--daemon-home`, `--fail-if-no-daemon`
- `serve`: `--daemon`, `--mode mcp`, `--idle-secs`, `--socket`
- `daemon start|stop|restart|status|list`
- Entire `crates/rgctl-mcp/` crate and MCP integration tests

**Migration:**

```bash
cd /path/to/repo
rgctl migrate-cache              # copy ~/.rgctl/cache/{reponame}/.rgctl/ if present
rgctl discover .                 # refresh artifacts in-repo
```

**Agent workflow:** spawn `rgctl -f json <command>` subprocesses (see [AGENTS.md](../../AGENTS.md)). Optional `rgctl serve` remains for local HTTP dashboard + `/api/query` on one repository.

**Unchanged:** `semantic index --embedder code-daemon` (ONNX embedder name — not related to removed daemon mode).

### `discover` repo root fix

`rgctl -r PATH discover .` no longer ignores `-r`; the positional `.` does not override the explicit repo root.

## Documentation

- User guide, installation, HTTP API, agent skill, and integration-test docs updated for CLI-first workflow.
- Removed [MCP Server guide](../guides/mcp-server.md).
