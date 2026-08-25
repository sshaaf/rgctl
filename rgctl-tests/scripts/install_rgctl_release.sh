#!/usr/bin/env bash
# Download and install rgctl from a GitHub Release (default: latest).
#
# Usage:
#   ./scripts/install_rgctl_release.sh
#   RGCTL_TAG=v0.1.0 ./scripts/install_rgctl_release.sh
#   RGCTL_INSTALL_DIR=/tmp/rgctl-bin ./scripts/install_rgctl_release.sh
#
# Writes the install directory to stdout (last line) and sets GITHUB_ENV when run in Actions.
set -euo pipefail

REPO="${RGCTL_REPO:-sshaaf/rgctl}"
TAG="${RGCTL_TAG:-}"
TARGET="${RGCTL_TARGET:-}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_DIR="${RGCTL_INSTALL_DIR:-$ROOT/.rgctl-bin}"

if [[ -z "$TARGET" ]]; then
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)  TARGET=x86_64-unknown-linux-gnu ;;
    Linux-aarch64|Linux-arm64) TARGET=aarch64-unknown-linux-gnu ;;
    Darwin-arm64)  TARGET=aarch64-apple-darwin ;;
    Darwin-x86_64) TARGET=x86_64-apple-darwin ;;
    *)
      echo "Unsupported platform: $(uname -s)-$(uname -m). Set RGCTL_TARGET explicitly." >&2
      exit 1
      ;;
  esac
fi

mkdir -p "$INSTALL_DIR"

AUTH=()
if [[ -n "${GITHUB_TOKEN:-}" ]]; then
  AUTH=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
elif [[ -n "${GH_TOKEN:-}" ]]; then
  AUTH=(-H "Authorization: Bearer ${GH_TOKEN}")
fi

if [[ -n "$TAG" ]]; then
  RELEASE_URL="https://api.github.com/repos/${REPO}/releases/tags/${TAG}"
else
  RELEASE_URL="https://api.github.com/repos/${REPO}/releases/latest"
fi

echo "Fetching release from ${REPO} (${TAG:-latest}) …" >&2
RELEASE_JSON="$(curl -fsSL "${AUTH[@]}" "$RELEASE_URL")"

read -r TAG_NAME VERSION <<< "$(python3 - <<'PY' "$RELEASE_JSON"
import json, sys
r = json.loads(sys.argv[1])
tag = r.get("tag_name") or ""
print(tag, tag.lstrip("v"))
PY
)"

if [[ -z "$VERSION" ]]; then
  echo "Could not resolve release tag from ${RELEASE_URL}" >&2
  echo "$RELEASE_JSON" | head -c 500 >&2
  exit 1
fi

ASSET="rgctl-${VERSION}-${TARGET}.tar.gz"
URL="$(python3 - <<'PY' "$RELEASE_JSON" "$ASSET"
import json, sys
r = json.loads(sys.argv[1])
want = sys.argv[2]
for a in r.get("assets", []):
    if a.get("name") == want:
        print(a["browser_download_url"])
        break
PY
)"

if [[ -z "$URL" ]]; then
  echo "Asset not found: ${ASSET} in release ${TAG_NAME}" >&2
  echo "Available assets:" >&2
  python3 - <<'PY' "$RELEASE_JSON" >&2
import json, sys
for a in json.loads(sys.argv[1]).get("assets", []):
    print(" ", a.get("name"))
PY
  exit 1
fi

TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT
echo "Downloading ${ASSET} …" >&2
curl -fsSL "${AUTH[@]}" -o "$TMP" "$URL"
tar -xzf "$TMP" -C "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/rgctl"

if ! "$INSTALL_DIR/rgctl" --version >/dev/null 2>&1; then
  "$INSTALL_DIR/rgctl" -h >/dev/null 2>&1 || true
fi

echo "Installed rgctl ${TAG_NAME} (${TARGET}) → ${INSTALL_DIR}/rgctl" >&2

if [[ -n "${GITHUB_ENV:-}" && -w "${GITHUB_ENV}" ]]; then
  {
    echo "RGCTL=${INSTALL_DIR}/rgctl"
    echo "RGCTL_VERSION=${TAG_NAME}"
    echo "RGCTL_TARGET=${TARGET}"
  } >> "$GITHUB_ENV"
fi

echo "$INSTALL_DIR"
