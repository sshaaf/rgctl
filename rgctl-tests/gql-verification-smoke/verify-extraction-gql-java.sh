#!/usr/bin/env bash
# Java extraction-depth GQL + rgctl command verification.
# GQL probes: tests/fixtures/java/langfeatures | Commands: ecommerce-java fixture
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RGCTL_TESTS="$(cd "${SCRIPT_DIR}/.." && pwd)"
source "${SCRIPT_DIR}/extraction-gql-common.sh"
RGCTL_CMD_ID=java
source "${SCRIPT_DIR}/rgctl-commands-config.sh"
source "${SCRIPT_DIR}/rgctl-commands-common.sh"

FIXTURE="${RGCTL_TESTS}/ecommerce-java"
LANGFEATURES="${RGCTL_MONOREPO}/tests/fixtures/java/langfeatures"
EXAMPLE_REL="example/metasfresh-4.9.8b"

run_langfeatures_gql() {
  [[ -d "${LANGFEATURES}" ]] || {
    echo "error: missing Java langfeatures fixture at ${LANGFEATURES}" >&2
    exit 1
  }
  echo "--- langfeatures GQL: ${LANGFEATURES} ---"
  discover_repo "${LANGFEATURES}"
  assert_gql_min "JF-01 instantiates (String)" \
    "MATCH (a:Function)-[:INSTANTIATES]->(b) WHERE a.name = 'instantiates' RETURN a,b" 1 "${LANGFEATURES}"
  assert_gql_min "JF-02 annotated with (NonNull)" \
    "MATCH (a:Function)-[:ANNOTATED_WITH]->(b) WHERE a.name = 'typeUse' RETURN a,b" 1 "${LANGFEATURES}"
  assert_gql_min "JF-03 references (field/class literal)" \
    "MATCH (a:Function)-[:REFERENCES]->(b) WHERE a.name = 'fieldAndClassLiteral' RETURN a,b" 1 "${LANGFEATURES}"
  assert_gql_min "JF-04 module depends on (JPMS)" \
    "MATCH (m:Module)-[:DEPENDSON]->(t) RETURN m,t" 1 "${LANGFEATURES}"
  assert_gql_min "JF-05 lambda (is_lambda)" \
    "MATCH (f:Function) WHERE f.is_lambda = 'true' RETURN f LIMIT 20" 1 "${LANGFEATURES}"
  assert_gql_min "JF-06 generic/throws properties" \
    "MATCH (f:Function) WHERE f.name = 'genericThrows' RETURN f" 1 "${LANGFEATURES}"
  assert_gql_min "JF-07 class FQN (qualified_name)" \
    "MATCH (n:Class) WHERE n.qualified_name = 'demo.LangFeatures' RETURN n" 1 "${LANGFEATURES}"
  assert_gql_min "JF-07 FQN LIKE filter" \
    "MATCH (n:Class) WHERE n.qualified_name LIKE 'demo.*' RETURN n" 1 "${LANGFEATURES}"
}

run_example_smoke() {
  [[ -z "${RGCTL_SKIP_EXAMPLE:-}" ]] || { echo "skip example smoke (RGCTL_SKIP_EXAMPLE set)"; return 0; }
  local ex="${RGCTL_MONOREPO}/${EXAMPLE_REL}"
  [[ -d "${ex}" ]] || { echo "skip example smoke (${EXAMPLE_REL} not cloned)"; return 0; }
  echo "--- example GQL: ${ex} ---"
  discover_repo "${ex}" --full
  assert_node_min "Class (scale)" Class 100 "${ex}"
  assert_edge_min "CALLS (scale)" CALLS 100 "${ex}"
  assert_edge_min "INSTANTIATES (scale)" INSTANTIATES 1 "${ex}"
}

echo "=== java extraction GQL + commands ==="
run_langfeatures_gql
discover_repo "${FIXTURE}" "${RGCTL_CMD_DISCOVER_EXTRA[@]}"
RGCTL_CMD_SKIP_DISCOVER=1 run_rgctl_commands_suite "${FIXTURE}"
run_example_smoke
echo "=== java extraction GQL + commands: OK ==="
