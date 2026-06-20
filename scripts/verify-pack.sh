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
node -e 'const p=require("./package.json"); console.log(`  Package: ${p.name}@${p.version}`)' 2>/dev/null || true
echo "  Tarball: $TARBALL"
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

# Confirm the protobufjs workaround shipped correctly: package must be bundled
# and its postinstall must be gone (guards against the npm@11 lifecycle-vs-
# extraction timing bug on `npm install -g --prefix DIR file.tgz`).
info "Verifying protobufjs bundle (postinstall workaround)…"
if tar -tf "$TARBALL" | grep -q 'node_modules/protobufjs/package.json'; then
  pass "protobufjs bundled in tarball"
else
  fail "protobufjs NOT found in tarball — postinstall workaround missing"
fi
PROTO_SCRIPTS="$(tar -xOf "$TARBALL" package/node_modules/protobufjs/package.json 2>/dev/null | node -e "process.stdin.setEncoding('utf8');let d='';process.stdin.on('data',c=>d+=c);process.stdin.on('end',()=>{try{const s=JSON.parse(d).scripts||{};console.log(s.postinstall||'null')}catch{console.log('parse-error')}})")"
if [ "$PROTO_SCRIPTS" = "null" ]; then
  pass "protobufjs bundled copy has no postinstall"
else
  fail "protobufjs bundled copy still has postinstall: $PROTO_SCRIPTS"
fi

# Global-prefix install — the exact path that triggered the protobufjs bug.
# Skipped automatically when npm is too old to support --prefix with -g reliably.
info "Testing global-prefix install (npm install -g --prefix)…"
GLOBAL_PREFIX="$TMP/global-prefix"
mkdir -p "$GLOBAL_PREFIX"
if npm install -g --prefix "$GLOBAL_PREFIX" --no-audit --no-fund --loglevel=error "$TARBALL" > "$TMP/global-install.log" 2>&1; then
  pass "npm install -g --prefix succeeded"
  GLOBAL_BIN="$GLOBAL_PREFIX/bin/hamr"
  if [ -f "$GLOBAL_BIN" ] || [ -L "$GLOBAL_BIN" ]; then
    if GVER="$("$GLOBAL_BIN" --version 2>/dev/null)"; then
      pass "global hamr --version → $GVER"
    else
      fail "global hamr --version failed"
    fi
  else
    fail "global hamr binary not found at $GLOBAL_BIN"
  fi
else
  fail "npm install -g --prefix failed:"; sed 's/^/      /' "$TMP/global-install.log"; echo ""
fi

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
