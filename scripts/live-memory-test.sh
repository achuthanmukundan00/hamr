#!/usr/bin/env bash
# live-memory-test.sh — Live LLM-driven memory tests using Relay + apodex-1.0-2b-sft-q4_k_m
#
# Tests the full memory pipeline: save_memory → search_memory → cross-session retrieval.
# Uses hamr's built-in relay config from ~/.hamr/agent/models.json — no manual
# header wrangling needed. A temp HAMR_MEMORY_DB isolates test data from your real
# ~/.hamr/memory.sqlite.
#
# Requirements:
#   - Relay running with apodex-1.0-2b-sft-q4_k_m loaded
#   - hamr built (`bun run build`)
#   - Env vars set: CF_ACCESS_CLIENT_ID, CF_ACCESS_CLIENT_SECRET (for CF Tunnel)
#     These are read by hamr from ~/.hamr/agent/models.json $VAR interpolation.
#
# Usage:
#   bash scripts/live-memory-test.sh [model]
#
#   Default model: apodex-1.0-2b-sft-q4_k_m

set -euo pipefail

GREEN='\033[0;32m'; RED='\033[0;31m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'; NC='\033[0m'
PASS=0; FAIL=0
pass() { echo -e "  ${GREEN}✓${NC} $1"; PASS=$((PASS + 1)); }
fail() { echo -e "  ${RED}✗${NC} $1"; FAIL=$((FAIL + 1)); }
warn() { echo -e "  ${YELLOW}⚠${NC} $1"; }
info() { echo -e "  ${BLUE}→${NC} $1"; }

MODEL="${1:-apodex-1.0-2b-sft-q4_k_m}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HAMR_BIN="node $ROOT/packages/coding-agent/dist/cli.js"

# ── Sanity checks ────────────────────────────────────────────────────

if [ ! -f "$ROOT/packages/coding-agent/dist/cli.js" ]; then
  echo -e "${RED}hamr not built. Run: bun run build${NC}"
  exit 1
fi

# The relay instance at ai.watchyourtemper.com requires CF_ACCESS env vars.
# The API key is handled by hamr's auth.json (configured via /login flow).
MISSING=()
for var in CF_ACCESS_CLIENT_ID CF_ACCESS_CLIENT_SECRET; do
  if [ -z "${!var:-}" ]; then MISSING+=("$var"); fi
done
if [ ${#MISSING[@]} -gt 0 ]; then
  echo -e "${RED}Missing env vars: ${MISSING[*]}${NC}"
  echo "Set them before running: export RELAY_API_KEY=..."
  exit 1
fi

# Quick connectivity check via hamr (uses auth.json for API key)
info "Checking relay connectivity via hamr…"
if $HAMR_BIN --list-models --provider relay 2>/dev/null | grep -qi "apodex\|$MODEL"; then
  pass "Relay reachable, model '$MODEL' found"
else
  warn "Could not confirm relay connectivity. Tests may fail."
  warn "Ensure the relay is running and /login was completed for the relay provider."
fi

# ── Isolated temp directory ──────────────────────────────────────────

TMP="$(mktemp -d)"
cleanup() { rm -rf "$TMP"; }
trap cleanup EXIT

export HAMR_MEMORY_DB="$TMP/memory.sqlite"
SESSION_DIR="$TMP/sessions"
mkdir -p "$SESSION_DIR"

echo ""
echo "============================================"
echo "  🧠 Live Memory Test"
echo "  Model:  $MODEL"
echo "  Memory: $HAMR_MEMORY_DB (isolated)"
echo "============================================"
echo ""

# ── Helper: run hamr in print mode ──────────────────────────────────

run_hamr() {
  local task="$1"
  local session_id="${2:-}"

  local args=(
    --print
    --provider relay
    --model "$MODEL"
    --session-dir "$SESSION_DIR"
  )
  if [ -n "$session_id" ]; then
    args+=(--session-id "$session_id")
  fi

  echo ""
  info "Task: $task"

  local stdout_file="$TMP/stdout.txt"
  local stderr_file="$TMP/stderr.txt"

  set +e
  $HAMR_BIN "${args[@]}" "$task" > "$stdout_file" 2> "$stderr_file"
  local exit_code=$?
  set -e

  if [ $exit_code -ne 0 ]; then
    echo -e "  ${RED}✗${NC} hamr exited with code $exit_code"
    [ -s "$stderr_file" ] && { echo "  stderr:"; sed 's/^/    /' "$stderr_file"; }
    return 1
  fi

  cat "$stdout_file"
  return 0
}

# ── Helper: assert output contains expected text ─────────────────────

assert_contains() {
  local output="$1"
  local pattern="$2"
  local label="$3"

  if echo "$output" | grep -qi "$pattern"; then
    pass "$label"
  else
    fail "$label (pattern '$pattern' not found)"
    echo -e "    ${YELLOW}Output tail:${NC}"
    echo "$output" | tail -15 | sed 's/^/      /'
    echo ""
  fi
}

# ── Test 1: save_memory ──────────────────────────────────────────────

echo "─── Test 1: save_memory ───"

UNIQUE_ID="memtest-$(date +%s)-$RANDOM"
FACT="Live test: the silver armadillo recites poetry under a blue moon. Ref: $UNIQUE_ID"

T1="$(run_hamr "Call save_memory with content: '$FACT'. Just call save_memory and report what happened. Do not call any other tools." || echo "FAILED")"

if [ "$T1" = "FAILED" ]; then
  fail "save_memory — hamr crashed"
else
  assert_contains "$T1" "saved\|memory\|📝\|stored" "save_memory — model confirmed save"
  if [ -f "$HAMR_MEMORY_DB" ] && [ -s "$HAMR_MEMORY_DB" ]; then
    pass "save_memory — memory.sqlite created and non-empty"
  else
    fail "save_memory — memory.sqlite missing or empty"
  fi
fi

# ── Test 2: search_memory (same isolated DB) ─────────────────────────

echo ""
echo "─── Test 2: search_memory ───"

T2="$(run_hamr "Call search_memory with query: 'silver armadillo poetry blue moon'. Report what you find. Only use search_memory, no other tools." || echo "FAILED")"

if [ "$T2" = "FAILED" ]; then
  fail "search_memory — hamr crashed"
else
  if echo "$T2" | grep -qi "silver\|armadillo\|poetry\|$UNIQUE_ID"; then
    pass "search_memory — found the saved fact"
  elif echo "$T2" | grep -qi "No memory result"; then
    fail "search_memory — returned 'No memory results' (data may not have been stored)"
  else
    fail "search_memory — model didn't report finding the fact"
  fi
fi

# ── Test 3: cross-session retrieval ──────────────────────────────────

echo ""
echo "─── Test 3: cross-session search ───"

T3="$(run_hamr "Call search_memory with query: 'silver armadillo poetry blue moon'. Report what you find. Only use search_memory." "cross-session-$RANDOM" || echo "FAILED")"

if [ "$T3" = "FAILED" ]; then
  fail "cross-session — hamr crashed"
else
  if echo "$T3" | grep -qi "silver\|armadillo\|poetry\|$UNIQUE_ID"; then
    pass "cross-session — found the fact from a different session"
  elif echo "$T3" | grep -qi "No memory result"; then
    fail "cross-session — returned 'No memory results' (cross-session search broken)"
  else
    fail "cross-session — model didn't report finding the fact"
  fi
fi

# ── Test 4: fact_store add + search ──────────────────────────────────

echo ""
echo "─── Test 4: fact_store ───"

FS_FACT="Structured fact: the copper penguin debugged the relay server at dawn. Ref: $UNIQUE_ID"

T4A="$(run_hamr "Call fact_store with action='add' and content='$FS_FACT' and tags='test,live'. Only call fact_store, nothing else." || echo "FAILED")"

if [ "$T4A" = "FAILED" ]; then
  fail "fact_store add — hamr crashed"
else
  assert_contains "$T4A" "fact.*id\|📌\|stored\|added\|Action completed" "fact_store add — confirmed"
fi

T4B="$(run_hamr "Call fact_store with action='search' and query='copper penguin debugged relay'. Report what you find. Only use fact_store." || echo "FAILED")"

if [ "$T4B" = "FAILED" ]; then
  fail "fact_store search — hamr crashed"
else
  if echo "$T4B" | grep -qi "copper\|penguin\|debugged\|$UNIQUE_ID"; then
    pass "fact_store search — found the structured fact"
  elif echo "$T4B" | grep -qi "No facts"; then
    fail "fact_store search — returned 'No facts found'"
  else
    fail "fact_store search — fact not found"
  fi
fi

# ── Test 5: cross-session fact_store search ──────────────────────────

echo ""
echo "─── Test 5: cross-session fact_store ───"

T5="$(run_hamr "Call fact_store with action='search' and query='copper penguin debugged relay'. Only use fact_store. Report what you find." "cross-fs-$RANDOM" || echo "FAILED")"

if [ "$T5" = "FAILED" ]; then
  fail "cross-session fact_store — hamr crashed"
else
  if echo "$T5" | grep -qi "copper\|penguin\|debugged\|$UNIQUE_ID"; then
    pass "cross-session fact_store — found across sessions"
  elif echo "$T5" | grep -qi "No facts"; then
    fail "cross-session fact_store — returned 'No facts found'"
  else
    fail "cross-session fact_store — not found across sessions"
  fi
fi

# ── Results ─────────────────────────────────────────────────────────

echo ""
echo "============================================"
TOTAL=$((PASS + FAIL))
if [ "$FAIL" -eq 0 ]; then
  echo -e "  ${GREEN}ALL $PASS TESTS PASSED${NC}"
else
  echo -e "  ${RED}$FAIL OF $TOTAL TESTS FAILED${NC}"
fi
echo "  DB size: $(ls -lh "$HAMR_MEMORY_DB" 2>/dev/null | awk '{print $5}' || echo 'N/A')"
echo "============================================"
echo ""

exit $FAIL
