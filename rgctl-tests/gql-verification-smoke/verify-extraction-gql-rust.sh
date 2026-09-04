#!/usr/bin/env bash
# Rust extraction-depth GQL + rgctl command verification.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=rust
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-rust"
EXAMPLE_REL="example/rust"

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_node_min "module graph (Import)" Import 1 "${FIXTURE}"
  assert_edge_min "trait heritage (IMPLEMENTS)" IMPLEMENTS 1 "${FIXTURE}"
  assert_edge_min "attributes (ANNOTATEDWITH)" ANNOTATEDWITH 1 "${FIXTURE}"
  assert_edge_min "instantiation (INSTANTIATES)" INSTANTIATES 1 "${FIXTURE}"
  assert_edge_min "call resolution (CALLS)" CALLS 50 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" -l rust
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "IMPLEMENTS (scale)" IMPLEMENTS 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== rust extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== rust extraction GQL + commands: OK ==="
