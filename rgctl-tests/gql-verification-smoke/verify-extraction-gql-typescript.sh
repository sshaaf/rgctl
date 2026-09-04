#!/usr/bin/env bash
# TypeScript extraction-depth GQL + rgctl command verification.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=typescript
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-typescript"
EXAMPLE_REL="example/vscode/src"

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_node_min "module graph (Import)" Import 1 "${FIXTURE}"
  assert_edge_min "heritage (EXTENDS)" EXTENDS 1 "${FIXTURE}"
  assert_edge_min "implements (IMPLEMENTS)" IMPLEMENTS 1 "${FIXTURE}"
  assert_edge_min "decorators (ANNOTATEDWITH)" ANNOTATEDWITH 1 "${FIXTURE}"
  assert_edge_min "call resolution (CALLS)" CALLS 1 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" -l typescript
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "IMPLEMENTS (scale)" IMPLEMENTS 1 "${ex}"
  assert_edge_min "EXTENDS (scale)" EXTENDS 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== typescript extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== typescript extraction GQL + commands: OK ==="
