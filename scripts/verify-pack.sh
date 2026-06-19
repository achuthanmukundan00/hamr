#!/usr/bin/env bash
# verify-pack.sh — smoke-test the @skaft/hamr tarball the way a user installs it.
#
# Installs the given tarball into a throwaway project with `npm install`
# (the path Bruce uses), then checks the bundled libs are present and the
# `hamr` CLI actually boots (--version, --help). This is what proves the
# bundledDependencies approach works end to end.
#
# Usage: bash scripts/verify-pack.sh [releases/skaft-hamr-<ver>.tgz]
# Exit 0 on pass, non-zero on failure.

set -euo pipefail

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; NC='\033[0m'
PASS=0; FAIL=0
pass() { echo -e "  ${GREEN}✓${NC} $1"; PASS=$((PASS + 1)); }
fail() { echo -e "  ${RED}✗${NC} $1"; FAIL=$((FAIL + 1)); }
info() { echo -e "  ${YELLOW}→${NC} $1"; }

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARBALL="${1:-}"
if [ -z "$TARBALL" ]; then
  TARBALL="$(ls -t "$ROOT"/releases/skaft-hamr-*.tgz 2>/dev/null | head -1 || true)"
fi
if [ -z "$TARBALL" ] || [ ! -f "$TARBALL" ]; then
  echo -e "${RED}No tarball found. Run scripts/build-release.mjs first.${NC}"; exit 1
fi
TARBALL="$(cd "$(dirname "$TARBALL")" && pwd)/$(basename "$TARBALL")"

echo ""
echo "============================================"
echo "  @skaft/hamr package verification"
echo "  $(basename "$TARBALL")"
echo "============================================"
echo ""

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT
info "Temp project: $TMP"

echo '{"name":"verify-pack-tmp","private":true}' > "$TMP/package.json"

info "Installing tarball with npm…"
if (cd "$TMP" && npm install --no-audit --no-fund --loglevel=error "$TARBALL" > "$TMP/install.log" 2>&1); then
  pass "npm install of tarball succeeded"
else
  fail "npm install failed:"; sed 's/^/      /' "$TMP/install.log"; echo ""
  echo -e "${RED}ABORTED${NC}"; exit 1
fi

PKG_DIR="$TMP/node_modules/@skaft/hamr"
BIN="$TMP/node_modules/.bin/hamr"

[ -f "$BIN" ] && pass "hamr CLI binary installed" || fail "hamr binary missing at node_modules/.bin/hamr"

for lib in tui ai agent; do
  if [ -f "$PKG_DIR/node_modules/@hamr/$lib/dist/index.js" ]; then
    pass "bundled @hamr/$lib present"
  else
    fail "bundled @hamr/$lib MISSING (bundle is broken)"
  fi
done

info "Booting the CLI…"
if VER_OUT="$("$BIN" --version 2>"$TMP/ver.err")"; then
  pass "hamr --version → $VER_OUT"
else
  fail "hamr --version failed:"; sed 's/^/      /' "$TMP/ver.err"
fi
if "$BIN" --help > /dev/null 2>"$TMP/help.err"; then
  pass "hamr --help"
else
  fail "hamr --help failed:"; sed 's/^/      /' "$TMP/help.err"
fi

# Guard against leaking source/config into the published package.
info "Checking package contents…"
for leak in src specs scripts .hamr.toml npm-shrinkwrap.json; do
  if [ -e "$PKG_DIR/$leak" ]; then fail "leaked '$leak' in package"; else pass "no '$leak' in package"; fi
done

echo ""
echo "============================================"
TOTAL=$((PASS + FAIL))
if [ "$FAIL" -eq 0 ]; then
  echo -e "  ${GREEN}ALL $PASS CHECKS PASSED${NC}"
  echo "============================================"; echo ""; exit 0
else
  echo -e "  ${RED}$FAIL OF $TOTAL CHECKS FAILED${NC}"
  echo "============================================"; echo ""; exit 1
fi
