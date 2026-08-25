#!/usr/bin/env bash
# Fetch all large local example repos used by profiling/testing (gitignored under /example).
# Includes:
# - linux (kernel)
# - kafka
# - metasfresh
# - coolstore-weblogic
# - kubernetes
# - k8s-website (kubernetes/website content/en via sparse checkout)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLE_DIR="$ROOT/example"
TMP_DIR="$EXAMPLE_DIR/.tmp-fetch"
mkdir -p "$EXAMPLE_DIR" "$TMP_DIR"

clone_if_missing() {
  local url="$1"
  local dest="$2"
  local depth="${3:-1}"
  if [[ -d "$dest/.git" ]]; then
    echo "Already present: $dest"
    return 0
  fi
  echo "Cloning: $url -> $dest"
  git clone --depth "$depth" "$url" "$dest"
}

clone_sparse_k8s_website_if_missing() {
  local dest="$1"
  local tmp="$TMP_DIR/k8s-website-clone"
  local url="https://github.com/kubernetes/website.git"
  if [[ -d "$dest/docs" || -f "$dest/search.md" ]]; then
    echo "Already present: $dest"
    return 0
  fi
  rm -rf "$tmp"
  echo "Cloning sparse kubernetes/website content/en -> $dest"
  git clone --depth 1 --filter=blob:none --sparse "$url" "$tmp"
  (
    cd "$tmp"
    git sparse-checkout set content/en
  )
  rm -rf "$dest"
  mv "$tmp/content/en" "$dest"
  rm -rf "$tmp"
}

# Full repos
clone_if_missing "https://github.com/torvalds/linux.git" "$EXAMPLE_DIR/linux"
clone_if_missing "https://github.com/apache/kafka.git" "$EXAMPLE_DIR/kafka"
clone_if_missing "https://github.com/metasfresh/metasfresh.git" "$EXAMPLE_DIR/metasfresh-4.9.8b"
clone_if_missing "https://github.com/konveyor-ecosystem/coolstore.git" "$EXAMPLE_DIR/coolstore-weblogic"
clone_if_missing "https://github.com/kubernetes/kubernetes.git" "$EXAMPLE_DIR/kubernetes"

# Sparse docs corpus
clone_sparse_k8s_website_if_missing "$EXAMPLE_DIR/k8s-website"

echo
echo "All requested example repos are available under: $EXAMPLE_DIR"
echo "Build: cargo build --release --bin rgctl"
echo "Cold profile gates:"
echo "  cargo test --release --test cold_profile_gates -- --ignored --nocapture --test-threads=1"
