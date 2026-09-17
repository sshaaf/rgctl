#!/usr/bin/env bash
# GQL smoke for Ruby extraction on ecommerce-ruby.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=extraction-gql-common.sh
source "$SCRIPT_DIR/extraction-gql-common.sh"

REPO="${RGCTL_ECOMMERCE_RUBY_REPO:-$SCRIPT_DIR/../../ecommerce-ruby}"
run_extraction_gql_smoke "$REPO" ruby
