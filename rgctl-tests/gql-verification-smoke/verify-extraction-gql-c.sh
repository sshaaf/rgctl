#!/usr/bin/env bash
# C extraction-depth GQL + rgctl command verification.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=c
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-c"
EXAMPLE_REL="example/linux"

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_edge_min "call resolution (CALLS)" CALLS 40 "${FIXTURE}"
  assert_gql_min "include graph (Import from .c)" "MATCH (n:Import) WHERE n.file_path LIKE '*.c' RETURN n LIMIT 10" 1 "${FIXTURE}"
  assert_gql_min "qualified symbols (file::symbol)" 'MATCH (n:Function) WHERE n.qualified_name LIKE "review_repository::*" RETURN n LIMIT 20' 1 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}"
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== c extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== c extraction GQL + commands: OK ==="
