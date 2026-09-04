#!/usr/bin/env bash
# Shared helpers for extraction-depth GQL verification scripts.
# Source from rgctl-tests/gql-verification-smoke/verify-extraction-gql-*.sh

set -euo pipefail

RGCTL_MONOREPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RGCTL_TESTS="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_rgctl() {
  if [[ -n "${RGCTL:-}" ]]; then
    if [[ -x "${RGCTL}" ]]; then
      echo "$(cd "$(dirname "${RGCTL}")" && pwd)/$(basename "${RGCTL}")"
      return
    fi
    if [[ -x "${RGCTL_MONOREPO}/${RGCTL}" ]]; then
      echo "${RGCTL_MONOREPO}/${RGCTL}"
      return
    fi
  fi
  for candidate in \
    "${RGCTL_MONOREPO}/target/release/rgctl" \
    "${RGCTL_MONOREPO}/target/debug/rgctl"; do
    if [[ -x "$candidate" ]]; then
      echo "$candidate"
      return
    fi
  done
  if command -v rgctl >/dev/null 2>&1; then
    command -v rgctl
    return
  fi
  echo "error: rgctl not found (build with: cargo build --release --bin rgctl)" >&2
  exit 1
}

gql_count() {
  local repo="$1"
  local query="$2"
  local rgctl
  rgctl="$(resolve_rgctl)"
  "$rgctl" -r "$repo" -f json gql "$query" 2>/dev/null | python3 -c '
import json, sys
d = json.load(sys.stdin)
c = d.get("count")
if c is None:
    c = len(d.get("rows") or [])
print(c)
' 2>/dev/null || echo 0
}

assert_gql_min() {
  local label="$1"
  local query="$2"
  local min="$3"
  local repo="$4"
  local count
  count="$(gql_count "$repo" "$query")"
  if [[ "$count" -lt "$min" ]]; then
    echo "FAIL: $label — expected >= $min, got $count" >&2
    echo "  query: $query" >&2
    return 1
  fi
  echo "ok: $label ($count >= $min)"
}

assert_node_min() {
  local label="$1"
  local node_label="$2"
  local min="$3"
  local repo="$4"
  assert_gql_min "$label" "MATCH (n:${node_label}) RETURN n LIMIT 10000" "$min" "$repo"
}

assert_edge_min() {
  local label="$1"
  local rel="$2"
  local min="$3"
  local repo="$4"
  assert_gql_min "$label" "MATCH (a)-[:${rel}]->(b) RETURN a,b LIMIT 10000" "$min" "$repo"
}

# Openspec probes that may not yet emit graph edges — warn instead of fail.
soft_assert_edge_min() {
  if ! assert_edge_min "$@"; then
    echo "warn: $1 (openspec probe — graph may not emit this edge yet)"
  fi
}

pick_discover_repo() {
  local example_rel="$1"
  local fixture="$2"
  shift 2
  local discover_args=("$@")
  if [[ -n "${RGCTL_USE_EXAMPLE:-}" ]]; then
    local example="${RGCTL_MONOREPO}/${example_rel}"
    if [[ ! -d "$example" ]]; then
      echo "error: RGCTL_USE_EXAMPLE set but missing ${example}" >&2
      echo "  fetch: ./scripts/fetch-profile-repos.sh" >&2
      exit 1
    fi
    echo "$example"
    return
  fi
  if [[ -n "${RGCTL_REPO:-}" ]]; then
    echo "${RGCTL_REPO}"
    return
  fi
  if [[ -d "${RGCTL_MONOREPO}/${example_rel}" ]]; then
    echo "${RGCTL_MONOREPO}/${example_rel}"
    return
  fi
  echo "$fixture"
}

discover_repo() {
  local repo="$1"
  shift
  local rgctl
  rgctl="$(resolve_rgctl)"
  echo "discover: $repo $*"
  rm -rf "$repo/.rgctl"
  (cd "$repo" && "$rgctl" -f json discover . "$@")
}

pick_repo() {
  local example_path="$1"
  local fixture_path="$2"
  if [[ -n "${RGCTL_REPO:-}" ]]; then
    echo "${RGCTL_REPO}"
  elif [[ -d "${RGCTL_MONOREPO}/${example_path}" ]]; then
    echo "${RGCTL_MONOREPO}/${example_path}"
  elif [[ -d "${fixture_path}" ]]; then
    echo "${fixture_path}"
  else
    echo "error: no repo at RGCTL_REPO, ${RGCTL_MONOREPO}/${example_path}, or ${fixture_path}" >&2
    exit 1
  fi
}

run_extraction_suite() {
  local lang="$1"
  local repo="$2"
  shift 2
  local discover_args=("$@")
  echo "=== extraction GQL: $lang ==="
  echo "repo: $repo"
  discover_repo "$repo" "${discover_args[@]}"
  return 0
}
