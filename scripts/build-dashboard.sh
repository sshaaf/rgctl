#!/usr/bin/env bash
# Build dashboard UI (Vite) and WASM engine for embed + local preview.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> wasm-pack (rgctl-wasm)"
if command -v wasm-pack >/dev/null 2>&1; then
  wasm-pack build "$ROOT/crates/rgctl-wasm" \
    --target web \
    --out-dir "$ROOT/dashboard/wasm" \
    --release \
    --no-opt
else
  echo "warn: wasm-pack not found — worker will use JS header fallback only"
  mkdir -p "$ROOT/dashboard/wasm"
  if [[ ! -f "$ROOT/dashboard/wasm/rgctl_wasm.js" ]]; then
    echo "export default async function init(){}" > "$ROOT/dashboard/wasm/rgctl_wasm.js"
    echo "export class EngineContext { constructor(){ throw new Error('wasm not built'); } }" >> "$ROOT/dashboard/wasm/rgctl_wasm.js"
  fi
fi

echo "==> npm run build (dashboard/)"
cd "$ROOT/dashboard"
npm install
npm run build

echo "==> done — dashboard/dist ready for rgctl-dashboard include_dir"
