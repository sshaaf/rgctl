#!/usr/bin/env bash
# Fail if stale rgBuilder/rgbuilder names appear outside allowlist.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

PATTERN='rgbuilder|rgBuilder|RGBUILDER_|RBUILDER_'

if rg -i "$PATTERN" \
  --glob '!.git/*' --glob '!target/*' --glob '!**/node_modules/**' \
  --glob '!docs/internal/rename-to-rgctl-plan.md' \
  --glob '!scripts/rename-to-rgctl.sh' \
  --glob '!scripts/rename-content.py' \
  --glob '!scripts/rename-audit.sh' \
  --glob '!docs/releases/v0.4.6.md' \
  --glob '!docs/releases/unreleased.md' \
  --glob '!.github/TASK_PLAN.md' \
  --glob '!crates/rgctl-graph/src/paths.rs' \
  --glob '!src/cli/daemon/config.rs' \
  -l . 2>/dev/null | head -30; then
  echo "FAIL: stale rename tokens found (showing up to 30 files)" >&2
  exit 1
fi

echo "OK: no stale rename tokens outside allowlist"
