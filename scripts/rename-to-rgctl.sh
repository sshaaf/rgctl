#!/usr/bin/env bash
# Mechanical rgctl → rgctl rename. Run from repo root.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== Phase 1: git mv directories =="

if [[ -d rgctl-macros && ! -d rgctl-macros ]]; then
  git mv rgctl-macros rgctl-macros
fi

for d in crates/rgctl-*; do
  [[ -d "$d" ]] || continue
  new="${d/rgctl/rgctl}"
  if [[ ! -d "$new" ]]; then
    git mv "$d" "$new"
  fi
done

if [[ -d skills/rgctl && ! -d skills/rgctl ]]; then
  git mv skills/rgctl skills/rgctl
fi

if [[ -d rgctl-tests && ! -d rgctl-tests ]]; then
  git mv rgctl-tests rgctl-tests
fi

if [[ -f rgctl-tests/rgctl-policy.json && ! -f rgctl-tests/rgctl-policy.json ]]; then
  git mv rgctl-tests/rgctl-policy.json rgctl-tests/rgctl-policy.json
fi

echo "== Phase 2: content replacements =="

# File types to process (exclude node_modules, target, .git)
find_args=(
  .
  \( -path ./.git -o -path ./target -o -path '*/node_modules/*' -o -path './rgctl-tests/*/node_modules/*' \) -prune
  -o -type f \( -name '*.rs' -o -name '*.toml' -o -name '*.md' -o -name '*.json' -o -name '*.yaml' -o -name '*.yml'
    -o -name '*.sh' -o -name '*.py' -o -name '*.tsx' -o -name '*.ts' -o -name '*.js' -o -name '*.mjs'
    -o -name '*.html' -o -name '*.txt' -o -name '*.tape' -o -name '*.lock' \)
  -print
)

replace_in_files() {
  local old="$1"
  local new="$2"
  local count=0
  while IFS= read -r f; do
    if grep -qF "$old" "$f" 2>/dev/null; then
      if [[ "$(uname)" == Darwin ]]; then
        sed -i '' "s/${old}/${new}/g" "$f"
      else
        sed -i "s/${old}/${new}/g" "$f"
      fi
      count=$((count + 1))
    fi
  done < <(eval "find ${find_args[*]}")
  echo "  $old → $new ($count files)"
}

# Order matters: most specific first
replace_in_files '.rgctl' '.rgctl'
replace_in_files 'RGCTL_' 'RGCTL_'
replace_in_files 'RGCTL_' 'RGCTL_'
replace_in_files 'rgctl' 'rgctl'
replace_in_files 'rgctl-' 'rgctl-'
replace_in_files 'rgctl_' 'rgctl_'
replace_in_files 'rgctl' 'rgctl'

echo "== Done =="
