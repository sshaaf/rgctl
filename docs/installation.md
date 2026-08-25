# Installation

Everything you need to install rgBuilder (`rgctl`), choose the right operating mode, and verify the setup works.

**Already installed?** Jump to [Choose your operating mode](#choose-your-operating-mode) or the [User Guide](user-guide.md).

---

## Table of contents

1. [Prerequisites](#prerequisites)
2. [Install rgctl](#install-rgctl)
3. [Add to PATH](#add-to-path)
4. [Verify the installation](#verify-the-installation)
5. [Choose your operating mode](#choose-your-operating-mode)
6. [Daemon vs no-daemon](#daemon-vs-no-daemon)
7. [Install the agent skill](#install-the-agent-skill)
8. [Optional: semantic search setup](#optional-semantic-search-setup)
9. [Upgrading](#upgrading)
10. [Uninstalling](#uninstalling)
11. [Troubleshooting](#troubleshooting)
12. [Next steps](#next-steps)

---

## Prerequisites

| Requirement | Notes |
|-------------|-------|
| **OS** | macOS (Apple Silicon or Intel), Linux (x86_64), Windows (x86_64) |
| **Rust 1.88+** | Only for building from source ([rustup.rs](https://rustup.rs/)). Pre-built binaries need no Rust toolchain. |
| **Git** | For cloning the repository (source builds) |
| **Git LFS** | Optional. Only required if you use `semantic index --embedder code-daemon` (~206 MB ONNX weights). The default `vocab` embedder needs no LFS. |

Disk space: the `.rgbuilder/` artifacts directory typically uses 50-500 MB depending on repository size and enabled features.

---

## Install rgctl

### Option A -- GitHub release (recommended)

Pre-built binaries are published on the project **Releases** page:

**https://github.com/sshaaf/rgBuilder/releases**

1. Open the latest release.
2. Download the archive for your platform:

   | Platform | Asset name |
   |----------|------------|
   | macOS (Apple Silicon) | `rgbuilder-*-aarch64-apple-darwin.tar.gz` |
   | macOS (Intel) | `rgbuilder-*-x86_64-apple-darwin.tar.gz` |
   | Linux (x86_64) | `rgbuilder-*-x86_64-unknown-linux-gnu.tar.gz` |
   | Windows | `rgbuilder-*-x86_64-pc-windows-msvc.zip` |

3. Extract the archive:

```bash
# macOS / Linux
tar -xzf rgbuilder-*-aarch64-apple-darwin.tar.gz
./rgctl --version
```

```powershell
# Windows (PowerShell)
Expand-Archive rgbuilder-*-x86_64-pc-windows-msvc.zip -DestinationPath .
.\rgctl.exe --version
```

### Option B -- Build from source

```bash
git clone https://github.com/sshaaf/rgBuilder.git
cd rgBuilder
cargo build --release --bin rgctl
./target/release/rgctl --version
```

All **nine** Tier 1 languages (Rust, Python, JavaScript, TypeScript, Go, Java, C#, C, C++) plus markdown are always included in the binary -- no per-language feature flags.

**Optional ONNX weights** (only for `--embedder code-daemon`):

```bash
git lfs pull   # ~206 MB; skip if using the default vocab embedder
```

---

## Add to PATH

### macOS / Linux -- user-local

```bash
mkdir -p ~/.local/bin
cp /path/to/rgctl ~/.local/bin/
chmod +x ~/.local/bin/rgctl
```

Add to `~/.zshrc` or `~/.bashrc`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Reload:

```bash
source ~/.zshrc   # or ~/.bashrc
```

### macOS / Linux -- system-wide

```bash
sudo cp /path/to/rgctl /usr/local/bin/
```

### Windows

1. Copy `rgctl.exe` to a folder such as `C:\Tools\rgctl\`.
2. Open **Settings > System > About > Advanced system settings > Environment Variables**.
3. Under **User variables**, edit `Path` and add `C:\Tools\rgctl`.
4. Open a new terminal.

### Per-project (no PATH change)

```bash
alias rgctl='/path/to/rgctl'
```

---

## Verify the installation

```bash
rgctl --version
```

Run a quick smoke test on any repository:

```bash
cd /path/to/any/repo
rgctl discover .
rgctl gql 'MATCH (n:Function) RETURN n LIMIT 5'
```

If both commands produce output without errors, the installation is working.

---

## Choose your operating mode

rgBuilder supports four operating modes. Pick the one that fits your workflow:

### CLI (one-shot commands)

The default. Run `discover` once, then issue queries as needed. Each command is a separate process.

```bash
rgctl discover .
rgctl -f json gql 'MATCH (n:Function) RETURN n LIMIT 10'
rgctl -f json blast-radius MyFunction
```

**Best for:** CI/CD pipelines, shell scripts, one-off queries, automation.

### HTTP server

A persistent HTTP server with an optional browser dashboard. Keeps the graph in memory for fast repeated queries.

```bash
rgctl serve --open              # starts on http://127.0.0.1:8080, opens browser
rgctl serve --port 3000         # custom port
rgctl serve --host 0.0.0.0     # bind all interfaces (team sharing)
rgctl serve --query-only        # API only, no dashboard
rgctl serve --no-pipeline       # serve existing artifacts, skip auto-pipeline
```

Query the API:

```bash
curl -s http://127.0.0.1:8080/api/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (f:Function) RETURN f LIMIT 5"}'
```

**Best for:** repeated queries in one session, team exploration, agent integration over HTTP, visual dashboard browsing.

See the [HTTP Server and Dashboard guide](guides/http-server-and-dashboard.md) and [HTTP API reference](http-api.md).

### MCP server (IDE integration)

A stdio-based MCP (Model Context Protocol) server for Cursor, Claude Code, and other MCP hosts. No HTTP -- the host spawns `rgctl` as a subprocess.

```bash
rgctl serve --mode mcp
```

The MCP server provides **seven tools**: `rgbuilder_query`, `rgbuilder_search`, `rgbuilder_impact`, `rgbuilder_metrics`, `rgbuilder_cpg`, `rgbuilder_check`, `rgbuilder_status`. It auto-runs the full pipeline on start (basic graph, CFG, dashboard, semantic index) unless `--no-pipeline` is passed.

**Configure Cursor** (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "rgbuilder": {
      "command": "rgctl",
      "args": ["-r", "/absolute/path/to/repo", "serve", "--mode", "mcp"]
    }
  }
}
```

**Configure Claude Code** (`.claude/settings.json`):

```json
{
  "mcpServers": {
    "rgbuilder": {
      "command": "rgctl",
      "args": ["-r", "/absolute/path/to/repo", "serve", "--mode", "mcp"]
    }
  }
}
```

**Best for:** IDE-integrated workflows where the agent queries the graph without leaving the editor.

See the [MCP Server guide](guides/mcp-server.md) for the full tool catalog and configuration details.

### Mode comparison

| | CLI | HTTP server | MCP server |
|---|---|---|---|
| **Transport** | Process per command | HTTP `127.0.0.1:8080` | stdio JSON-RPC |
| **Dashboard** | No | Yes (optional) | No (artifacts on disk only) |
| **Auto pipeline** | No (manual `discover`) | Yes (unless `--no-pipeline`) | Yes (unless `--no-pipeline`) |
| **Warm graph** | No (loads from snapshot) | Yes (in-memory) | Yes (in-memory) |
| **Use case** | CI, scripts, one-off | Team, repeated queries, visual | IDE agents (Cursor, Claude Code) |
| **Output** | stdout (text or `-f json`) | HTTP JSON responses | JSON-RPC tool results |

---

## Daemon vs no-daemon

By default, `rgctl` uses a background daemon. Indexed artifacts are cached under `~/.rgbuilder/` (override with `--daemon-home` or `RGCTL_HOME`).

Use `--no-daemon` to store artifacts in the repository itself at `{repo}/.rgbuilder/`:

```bash
rgctl --no-daemon discover .
```

| | Default (daemon) | `--no-daemon` |
|---|---|---|
| **Artifact location** | `~/.rgbuilder/cache/{reponame}/` | `{repo}/.rgbuilder/` |
| **Shared across sessions** | Yes | No (repo-local) |
| **Best for** | Interactive development | CI, containers, cold profiles, reproducible builds |

Add `.rgbuilder/` to your `.gitignore` when using `--no-daemon`.

---

## Install the agent skill

After `rgctl` is on your PATH, install the bundled skill into the target repository:

```bash
rgctl install --skill                    # current directory
rgctl -r /path/to/repo install --skill   # specific repo
```

This writes skill files to:

- `<repo>/.claude/skills/rgbuilder/` (Claude Code)
- `<repo>/.cursor/skills/rgbuilder/` (Cursor)

Limit to one host with `--host claude` or `--host cursor`. Use `--force` to overwrite after upgrading `rgctl`.

See the [Agent Skill guide](guides/agent-skill.md) and [AGENTS.md](../AGENTS.md).

---

## Optional: semantic search setup

Semantic search is **not** part of `discover` -- it requires a separate indexing step:

```bash
rgctl semantic index                          # default vocab embedder (no LFS needed)
rgctl -f json semantic query "checkout flow"  # search
```

**Embedder options:**

| Embedder | Command | Requirements | Quality |
|----------|---------|-------------|---------|
| `vocab` (default) | `semantic index` | None | Good (compiled token table) |
| `hash` | `semantic index --embedder hash` | None | Fast, lower quality (CI/testing) |
| `code-daemon` | `semantic index --embedder code-daemon` | Git LFS (~206 MB ONNX) | Best |

For document section search: `semantic index --scope docs --embedder hash`.

See the [Semantic Search guide](guides/semantic-search.md).

---

## Upgrading

### From a release binary

Download the new release from [GitHub Releases](https://github.com/sshaaf/rgBuilder/releases) and replace the old binary:

```bash
cp /path/to/new/rgctl ~/.local/bin/rgctl
chmod +x ~/.local/bin/rgctl
rgctl --version
```

After upgrading, refresh agent skills in each repository:

```bash
rgctl install --skill --force
```

### From source

```bash
git pull
cargo build --release --bin rgctl
```

### Re-index after upgrading

The `.rgbuilder/` snapshot format may change between versions. Re-run `discover` after upgrading:

```bash
rgctl discover .
```

---

## Uninstalling

1. Remove the binary:

```bash
rm ~/.local/bin/rgctl          # or wherever you placed it
# Windows: delete rgctl.exe from C:\Tools\rgctl\
```

2. Remove cached artifacts (optional):

```bash
rm -rf ~/.rgbuilder            # daemon cache
# Per-repo artifacts (if --no-daemon was used):
rm -rf /path/to/repo/.rgbuilder
```

3. Remove agent skill files (optional):

```bash
rm -rf /path/to/repo/.claude/skills/rgbuilder
rm -rf /path/to/repo/.cursor/skills/rgbuilder
```

4. Remove the PATH entry from your shell profile if you added one.

---

## Troubleshooting

### `rgctl: command not found`

The binary is not on your PATH. Verify:

```bash
which rgctl          # macOS / Linux
where.exe rgctl      # Windows
```

If empty, revisit [Add to PATH](#add-to-path). For GUI apps (Cursor, VS Code), note that they may not inherit your shell PATH -- use absolute paths in MCP config.

### `discover` fails or produces no output

- Ensure you are in a directory with source files in a [supported language](languages.md).
- Check `rgctl --version` works first.
- Try verbose mode: `rgctl discover . -v`
- For detailed timing: `RUST_LOG=info rgctl discover . -v`

### Queries fail with "no graph found"

Run `discover` first. All query commands (`gql`, `blast-radius`, `metrics`, etc.) require a prior `discover`.

### MCP tools return pipeline status instead of results

The pipeline is still running. Wait for it to complete. Check status with the `rgbuilder_status` tool or look at stderr output.

### Slow `discover` on large repositories

Start with the default mode (no extra flags). Add `--with-cfg`, `--with-taint`, `--with-dashboard`, `--with-harmonic` only when you need those features. See [User Guide -- Troubleshooting](user-guide.md#18-troubleshooting) for tuning large repos.

### Build from source fails

- Confirm Rust 1.88+: `rustc --version`
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build --release --bin rgctl`

---

## Next steps

| Goal | Where to go |
|------|-------------|
| Full CLI walkthrough | [User Guide](user-guide.md) |
| Concepts and architecture | [Introduction](Introduction.md) |
| Agent workflows | [AGENTS.md](../AGENTS.md) |
| Step-by-step feature guides | [Guides](guides/README.md) |
| JSON output reference | [JSON API](json-api.md) |
| HTTP API details | [HTTP API](http-api.md) |
| Supported languages | [Languages](languages.md) |
