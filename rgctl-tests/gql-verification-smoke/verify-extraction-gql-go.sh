#!/usr/bin/env bash
# Go extraction-depth GQL + rgctl command verification.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=go
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-go"
EXAMPLE_REL="example/kubernetes"

run_fixture_gql() {
  echo "--- fixture GQL: ${FIXTURE} ---"
  discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
  assert_gql_min "LF-05 implements (LfRemoteRuntime)" \
    "MATCH (a:Struct)-[:IMPLEMENTS]->(b:Interface) WHERE a.name = 'LfRemoteRuntime' RETURN a,b" 1 "${FIXTURE}"
  assert_gql_min "LF-06 embed extends (LfDerived)" \
    "MATCH (a:Struct)-[:EXTENDS]->(b:Struct) WHERE a.name = 'LfDerived' RETURN a,b" 1 "${FIXTURE}"
  assert_gql_min "LF-10 const (LfStatusPending)" \
    "MATCH (n:Variable) WHERE n.name = 'LfStatusPending' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "LF-10 type alias (LfUserID)" \
    "MATCH (n:TypeAlias) WHERE n.name = 'LfUserID' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "LF-16 generics (LfIdentity)" \
    "MATCH (n:Function) WHERE n.name = 'LfIdentity' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "LF-16 generics (LfBox)" \
    "MATCH (n:Struct) WHERE n.name = 'LfBox' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "LF-17 import (fmt)" \
    "MATCH (n:Import) WHERE n.name = 'fmt' RETURN n" 1 "${FIXTURE}"
  assert_gql_min "LF-17 import (timeutil)" \
    "MATCH (n:Import) WHERE n.name = 'timeutil' RETURN n" 1 "${FIXTURE}"
  assert_edge_min "call resolution (CALLS)" CALLS 1 "${FIXTURE}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" -l go -e vendor
  assert_node_min "Import (scale)" Import 1 "${ex}"
  assert_edge_min "IMPLEMENTS (scale)" IMPLEMENTS 1 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
}

echo "=== go extraction GQL + commands ==="
run_fixture_gql
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== go extraction GQL + commands: OK ==="
