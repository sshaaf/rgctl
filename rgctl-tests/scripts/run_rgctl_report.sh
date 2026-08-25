#!/usr/bin/env bash
# Run rgctl feature matrix and publish markdown + HTML reports.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec python3 "$ROOT/scripts/run_rgctl_report.py" "$@"
