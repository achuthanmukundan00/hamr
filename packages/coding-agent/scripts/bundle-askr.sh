#!/usr/bin/env bash
# Bundle askr skills into dist/askr/skills/ at build time.
# Fetches the latest askr release tag and copies skills.
set -euo pipefail

ASKR_REPO="git@github.com:skaft-software/askr.git"
DIST_ASKR_DIR="dist/askr/skills"

# Resolve the latest release tag, fall back to main
echo "[bundle-askr] fetching latest askr release..."
LATEST_TAG=$(git ls-remote --tags --sort=-version:refname "$ASKR_REPO" 'v*' 2>/dev/null | head -1 | awk '{print $2}' | sed 's|refs/tags/||')

if [ -z "$LATEST_TAG" ]; then
  echo "[bundle-askr] no release tags found, using main branch"
  REF="main"
else
  echo "[bundle-askr] latest release: $LATEST_TAG"
  REF="$LATEST_TAG"
fi

TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

git clone --depth 1 --branch "$REF" "$ASKR_REPO" "$TEMP_DIR"

rm -rf "$DIST_ASKR_DIR"
mkdir -p "$(dirname "$DIST_ASKR_DIR")"
cp -r "$TEMP_DIR/skills" "$DIST_ASKR_DIR"

echo "[bundle-askr] done — $(ls -1 "$DIST_ASKR_DIR" | wc -l) skills bundled"
