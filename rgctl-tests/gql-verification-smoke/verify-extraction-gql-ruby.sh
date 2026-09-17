#!/usr/bin/env bash
# Ruby extraction-depth GQL + rgctl command verification.
# Fixture: rgctl-tests/ecommerce-ruby | Example: example/discourse (-l ruby)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
# shellcheck source=extraction-gql-common.sh
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=ruby
# shellcheck source=rgctl-commands-config.sh
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
# shellcheck source=rgctl-commands-common.sh
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-ruby"
EXAMPLE_REL="example/discourse"

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_node_min "require graph (Import)" Import 1 "${FIXTURE}"
  assert_edge_min "mixin heritage (EXTENDS)" EXTENDS 1 "${FIXTURE}"
  assert_edge_min "instantiation (INSTANTIATES)" INSTANTIATES 1 "${FIXTURE}"
  assert_edge_min "call resolution (CALLS)" CALLS 1 "${FIXTURE}"
  assert_gql_min "method FQN (OrderService#process)" \
    "MATCH (n:Function) WHERE n.qualified_name = 'OrderService#process' RETURN n LIMIT 5" 1 "${FIXTURE}"
  assert_gql_min "constructor FQN (OrderDTO.<init>)" \
    "MATCH (n:Function) WHERE n.qualified_name = 'OrderDTO.<init>' RETURN n LIMIT 5" 1 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" -l ruby -e vendor,tmp,node_modules
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== ruby extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== ruby extraction GQL + commands: OK ==="
