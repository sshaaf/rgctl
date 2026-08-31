#!/usr/bin/env bash
# Initialize the Konveyor rulesets submodule for rgctl-kantra embedded catalog builds.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUBMODULE="$ROOT/crates/rgctl-kantra/assets/rulesets"
PIN_TAG="${KANTRA_RULESETS_TAG:-v0.9.2}"

if [[ ! -f "$ROOT/.gitmodules" ]]; then
  echo "error: run from rgctl repository root" >&2
  exit 1
fi

git -C "$ROOT" submodule update --init crates/rgctl-kantra/assets/rulesets

if [[ -d "$SUBMODULE/.git" || -f "$SUBMODULE/.git" ]]; then
  echo "Checking out rulesets tag: $PIN_TAG"
  git -C "$SUBMODULE" fetch --tags --depth 1 origin 2>/dev/null || git -C "$SUBMODULE" fetch --tags origin
  git -C "$SUBMODULE" checkout "$PIN_TAG"
  echo "rulesets HEAD: $(git -C "$SUBMODULE" rev-parse HEAD)"
else
  echo "error: submodule path missing after init: $SUBMODULE" >&2
  exit 1
fi

echo "Done. Build with: cargo build -p rgctl-kantra"
