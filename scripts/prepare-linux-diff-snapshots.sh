#!/usr/bin/env bash
# Prepare linux columnar snapshots for cold *diff* profiling (not cold discover).
#
# Pair (default): BASE_REF=v7.1 vs HEAD_REF=HEAD of example/linux.
# Writes:
#   example/linux/.rgctl-diff/base/graph.snapshot.bin
#   example/linux/.rgctl-diff/head/graph.snapshot.bin
#   example/linux/.rgctl-diff/meta.env
#
# Prerequisites:
#   - cargo build --release --bin rgctl
#   - example/linux checkout with enough history for BASE_REF (shallow depth=1
#     clones need: git -C example/linux fetch --depth 1 origin tag v7.1)
#
# Usage:
#   ./scripts/prepare-linux-diff-snapshots.sh
#   BASE_REF=v6.1 HEAD_REF=v6.6 ./scripts/prepare-linux-diff-snapshots.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LINUX="${RGCTL_LINUX_REPO:-$ROOT/example/linux}"
BASE_REF="${BASE_REF:-v7.1}"
HEAD_REF="${HEAD_REF:-HEAD}"
RGCTL="${RGCTL_BIN:-$ROOT/target/release/rgctl}"
WORK="${LINUX}/.rgctl-diff-worktrees"
OUT="${LINUX}/.rgctl-diff"

if [[ ! -d "$LINUX/.git" ]]; then
  echo "error: linux checkout missing at $LINUX (run ./scripts/fetch-profile-repos.sh)" >&2
  exit 1
fi
if [[ ! -x "$RGCTL" ]]; then
  echo "error: rgctl binary not found at $RGCTL — run: cargo build --release --bin rgctl" >&2
  exit 1
fi

resolve_ref() {
  local ref="$1"
  if ! git -C "$LINUX" rev-parse --verify "$ref^{commit}" >/dev/null 2>&1; then
    echo "Fetching missing ref: $ref"
    if [[ "$ref" == HEAD ]]; then
      echo "error: HEAD unresolvable in $LINUX" >&2
      exit 1
    fi
    # Prefer a shallow tag fetch; fall back to unshallow tip + tag.
    if ! git -C "$LINUX" fetch --depth 1 origin "refs/tags/${ref}:refs/tags/${ref}" 2>/dev/null; then
      git -C "$LINUX" fetch --tags --depth 1 origin "+refs/tags/${ref}:refs/tags/${ref}" || true
    fi
  fi
  if ! git -C "$LINUX" rev-parse --verify "$ref^{commit}" >/dev/null 2>&1; then
    echo "error: cannot resolve $ref in $LINUX" >&2
    echo "hint: git -C \"$LINUX\" fetch origin tag $ref" >&2
    exit 1
  fi
  git -C "$LINUX" rev-parse --verify "$ref^{commit}"
}

discover_into() {
  local src="$1"
  local dest="$2"
  local label="$3"
  rm -rf "${src}/.rgctl"
  echo "==> discover ($label) in $src"
  (
    cd "$src"
    RUST_LOG=info,profile=info "$RGCTL" -f json discover . -v
  )
  mkdir -p "$dest"
  cp -f "${src}/.rgctl/graph.snapshot.bin" "${dest}/graph.snapshot.bin"
  echo "    wrote ${dest}/graph.snapshot.bin"
}

BASE_SHA="$(resolve_ref "$BASE_REF")"
HEAD_SHA="$(resolve_ref "$HEAD_REF")"

echo "BASE_REF=$BASE_REF ($BASE_SHA)"
echo "HEAD_REF=$HEAD_REF ($HEAD_SHA)"

rm -rf "$WORK" "$OUT"
mkdir -p "$WORK" "$OUT/base" "$OUT/head"

BASE_WT="$WORK/base"
HEAD_WT="$WORK/head"

# Detach worktrees so we never disturb the main checkout's index.
git -C "$LINUX" worktree remove --force "$BASE_WT" 2>/dev/null || true
git -C "$LINUX" worktree remove --force "$HEAD_WT" 2>/dev/null || true
git -C "$LINUX" worktree add --detach "$BASE_WT" "$BASE_SHA"
git -C "$LINUX" worktree add --detach "$HEAD_WT" "$HEAD_SHA"

discover_into "$BASE_WT" "$OUT/base" "base/$BASE_REF"
discover_into "$HEAD_WT" "$OUT/head" "head/$HEAD_REF"

cat >"$OUT/meta.env" <<EOF
BASE_REF=$BASE_REF
HEAD_REF=$HEAD_REF
BASE_SHA=$BASE_SHA
HEAD_SHA=$HEAD_SHA
PREPARED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF

# Drop worktrees (keep snapshots only).
git -C "$LINUX" worktree remove --force "$BASE_WT" || true
git -C "$LINUX" worktree remove --force "$HEAD_WT" || true
rm -rf "$WORK"

echo
echo "Ready. Cold diff:"
echo "  RUST_LOG=info,profile=info $RGCTL -f json diff \\"
echo "    --base $OUT/base --head $OUT/head"
echo
echo "Or gate:"
echo "  cargo test --release --test cold_profile_gates linux_cold_diff_within_baseline -- --ignored --nocapture"
