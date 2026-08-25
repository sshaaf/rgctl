#!/usr/bin/env bash
# Record markdown context graph CLI demo (VHS).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../" && pwd)"
cd "$ROOT"
if [[ ! -x target/debug/rgctl ]]; then
  cargo build --bin rgctl
fi
export PATH="$ROOT/target/debug:$PATH"
vhs docs/videos/markdown-context-cli.tape
