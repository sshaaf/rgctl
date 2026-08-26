#!/usr/bin/env bash
# Smoke-test rgctl MCP through the OpenCode CLI (Tier C host integration).
#
# Requires: opencode on PATH, built rgctl, tests/fixtures/tiny_polyglot_repo
#
# Usage:
#   ./scripts/integration/opencode-mcp-smoke.sh
#   RGCTL_OPENCODE_MODE=daemon ./scripts/integration/opencode-mcp-smoke.sh
#   RGCTL_REQUIRE_OPENCODE=1 ./scripts/integration/opencode-mcp-smoke.sh  # fail if opencode missing
#
# See docs/internal/integration-tests.md for full Tier A/B/C matrix.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
FIXTURE="$REPO_ROOT/tests/fixtures/tiny_polyglot_repo"
MODE="${RGCTL_OPENCODE_MODE:-stdio}"
REQUIRE_OPENCODE="${RGCTL_REQUIRE_OPENCODE:-0}"

RGCTL="${RGCTL_RGCTL:-$REPO_ROOT/target/debug/rgctl}"
if [[ ! -x "$RGCTL" ]]; then
  echo "[opencode-smoke] building rgctl…" >&2
  cargo build --bin rgctl -q --manifest-path "$REPO_ROOT/Cargo.toml"
  RGCTL="$REPO_ROOT/target/debug/rgctl"
fi

if [[ ! -d "$FIXTURE" ]]; then
  echo "error: fixture missing at $FIXTURE" >&2
  exit 1
fi

if ! command -v opencode >/dev/null 2>&1; then
  if [[ "$REQUIRE_OPENCODE" == "1" ]]; then
    echo "error: opencode not on PATH (install from https://opencode.ai)" >&2
    exit 1
  fi
  echo "skip: opencode not on PATH" >&2
  exit 0
fi

SCRATCH="$(mktemp -d /tmp/rgctl-opencode-smoke.XXXXXX)"
DAEMON_HOME=""
PORT=""

cleanup() {
  if [[ -n "$DAEMON_HOME" && -x "$RGCTL" ]]; then
    "$RGCTL" --daemon-home "$DAEMON_HOME" daemon stop >/dev/null 2>&1 || true
  fi
  rm -rf "$SCRATCH"
}
trap cleanup EXIT

mkdir -p "$SCRATCH/repo"
cp -R "$FIXTURE/." "$SCRATCH/repo/"
rm -rf "$SCRATCH/repo/.rgctl" "$SCRATCH/repo/.rbuilder"

echo "[opencode-smoke] indexing tiny fixture (no-daemon)…" >&2
(
  cd "$SCRATCH/repo"
  "$RGCTL" --no-daemon discover . --languages java,rust >/dev/null
)

write_stdio_config() {
  cat >"$SCRATCH/opencode.json" <<EOF
{
  "\$schema": "https://opencode.ai/config.json",
  "mcp": {
    "rgctl": {
      "type": "local",
      "command": ["${RGCTL}", "--no-daemon", "serve", "--mode", "mcp", "--no-pipeline"],
      "cwd": "repo",
      "enabled": true,
      "timeout": 60000,
      "environment": {
        "RGCTL_NO_DAEMON": "1",
        "RUST_LOG": "error"
      }
    }
  }
}
EOF
}

write_daemon_config() {
  DAEMON_HOME="$SCRATCH/daemon-home"
  mkdir -p "$DAEMON_HOME"
  PORT="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
  echo "[opencode-smoke] starting daemon on 127.0.0.1:${PORT}…" >&2
  "$RGCTL" --daemon-home "$DAEMON_HOME" daemon start --host 127.0.0.1 --port "$PORT" >/dev/null
  (
    cd "$SCRATCH/repo"
    "$RGCTL" --daemon-home "$DAEMON_HOME" discover . >/dev/null
  )
  cat >"$SCRATCH/opencode.json" <<EOF
{
  "\$schema": "https://opencode.ai/config.json",
  "mcp": {
    "rgctl": {
      "type": "local",
      "command": ["${RGCTL}", "--daemon-home", "${DAEMON_HOME}", "serve", "--mode", "mcp"],
      "cwd": "repo",
      "enabled": true,
      "timeout": 60000,
      "environment": {
        "RGCTL_HOME": "${DAEMON_HOME}",
        "RUST_LOG": "error"
      }
    }
  }
}
EOF
}

case "$MODE" in
  stdio) write_stdio_config ;;
  daemon) write_daemon_config ;;
  *)
    echo "error: unknown RGCTL_OPENCODE_MODE=$MODE (use stdio or daemon)" >&2
    exit 1
    ;;
esac

echo "[opencode-smoke] opencode mcp list (mode=$MODE)…" >&2
set +e
OUTPUT="$(cd "$SCRATCH" && opencode mcp list 2>&1)"
STATUS=$?
set -e

printf '%s\n' "$OUTPUT"

if [[ $STATUS -ne 0 ]]; then
  echo "error: opencode mcp list exited $STATUS" >&2
  exit 1
fi

if printf '%s\n' "$OUTPUT" | grep -Eiq 'rgctl.*connected|✓[^|]*rgctl'; then
  echo "[opencode-smoke] OK — rgctl connected" >&2
  exit 0
fi

if printf '%s\n' "$OUTPUT" | grep -Eiq 'rgctl.*failed|✗[^|]*rgctl'; then
  echo "error: opencode reports rgctl MCP failed" >&2
  exit 1
fi

echo "error: could not confirm rgctl MCP status in opencode output" >&2
exit 1
