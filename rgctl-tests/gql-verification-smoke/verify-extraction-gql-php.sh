#!/usr/bin/env bash
# PHP extraction-depth GQL + rgctl command verification.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=php
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-php"
EXAMPLE_REL="example/magento2"
EXAMPLE_DISCOVER=(app lib setup -l php -e vendor -e generated)

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_node_min "namespace imports (Import)" Import 1 "${FIXTURE}"
  assert_gql_min "import by name (AuthService)" \
    "MATCH (n:Import) WHERE n.name = 'AuthService' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "aliased import (Order)" \
    "MATCH (n:Import) WHERE n.name = 'Order' RETURN n" 1 "${FIXTURE}"
  assert_edge_min "call resolution (CALLS)" CALLS 1 "${FIXTURE}"
  assert_gql_min "cross-file static call (SampleService -> AuthService.login)" \
    "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE a.name = 'run' AND b.name = 'login' RETURN a,b" 1 "${FIXTURE}"
  assert_gql_min "namespace FQN on Class" \
    "MATCH (n:Class) WHERE n.name = 'AuthService' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "method FQN (AuthService.login)" \
    "MATCH (n:Function) WHERE n.name = 'login' RETURN n" 1 "${FIXTURE}"
  soft_assert_edge_min "trait composition (USES)" USES 1 "${FIXTURE}"
  soft_assert_edge_min "attributes (ANNOTATEDWITH)" ANNOTATEDWITH 1 "${FIXTURE}"
  soft_assert_edge_min "anonymous class / new (INSTANTIATES)" INSTANTIATES 1 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" "${EXAMPLE_DISCOVER[@]}"
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== php extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== php extraction GQL + commands: OK ==="
