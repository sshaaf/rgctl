#!/usr/bin/env bash
# Run all extraction-depth GQL + rgctl command verification scripts.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

LANGS=(
  c
  cpp
  csharp
  go
  java
  javascript
  php
  python
  rust
  typescript
)

failed=0
for lang in "${LANGS[@]}"; do
  script="${SCRIPT_DIR}/verify-extraction-gql-${lang}.sh"
  if [[ ! -x "${script}" ]]; then
    chmod +x "${script}"
  fi
  echo ""
  if ! "${script}"; then
    echo "FAILED: ${lang}" >&2
    failed=$((failed + 1))
  fi
done

echo ""
if [[ "${failed}" -gt 0 ]]; then
  echo "${failed} language(s) failed" >&2
  exit 1
fi
echo "All extraction GQL scripts passed."
