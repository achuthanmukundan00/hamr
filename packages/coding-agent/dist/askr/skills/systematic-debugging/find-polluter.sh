#!/usr/bin/env bash
# Linear search script to find which test creates unwanted files/state.
# Usage:
#   ./find-polluter.sh <file_or_dir_to_check> <name_pattern>
#   ./find-polluter.sh <file_or_dir_to_check> -path <path_pattern>
# Examples:
#   ./find-polluter.sh '.git' '*.test.ts'
#   ./find-polluter.sh '.git' -path '*src/*.test.ts'

set -euo pipefail

usage() {
  echo "Usage: $0 <file_to_check> <name_pattern>"
  echo "   or: $0 <file_to_check> -path <path_pattern>"
  echo "Example: $0 '.git' '*.test.ts'"
  echo "Example: $0 '.git' -path '*src/*.test.ts'"
}

if [ $# -eq 2 ]; then
  POLLUTION_CHECK="$1"
  MATCH_MODE="-name"
  TEST_PATTERN="$2"
elif [ $# -eq 3 ] && { [ "$2" = "-name" ] || [ "$2" = "-path" ]; }; then
  POLLUTION_CHECK="$1"
  MATCH_MODE="$2"
  TEST_PATTERN="$3"
else
  usage
  exit 1
fi

echo "🔍 Searching for test that creates: $POLLUTION_CHECK"
echo "Test match: $MATCH_MODE $TEST_PATTERN"
echo ""

TEST_FILES=$(find . "$MATCH_MODE" "$TEST_PATTERN" -type f | sort)
TOTAL=$(printf '%s\n' "$TEST_FILES" | sed '/^$/d' | wc -l | tr -d ' ')

echo "Found $TOTAL test files"
echo ""

COUNT=0
while IFS= read -r TEST_FILE; do
  [ -n "$TEST_FILE" ] || continue
  COUNT=$((COUNT + 1))

  if [ -e "$POLLUTION_CHECK" ]; then
    echo "⚠️  Pollution already exists before test $COUNT/$TOTAL"
    echo "   Skipping: $TEST_FILE"
    continue
  fi

  echo "[$COUNT/$TOTAL] Testing: $TEST_FILE"
  npm test "$TEST_FILE" > /dev/null 2>&1 || true

  if [ -e "$POLLUTION_CHECK" ]; then
    echo ""
    echo "🎯 FOUND POLLUTER!"
    echo "   Test: $TEST_FILE"
    echo "   Created: $POLLUTION_CHECK"
    echo ""
    echo "Pollution details:"
    ls -la "$POLLUTION_CHECK"
    echo ""
    echo "To investigate:"
    echo "  npm test $TEST_FILE    # Run just this test"
    echo "  cat $TEST_FILE         # Review test code"
    exit 1
  fi
done <<< "$TEST_FILES"

echo ""
echo "✅ No polluter found - all tests clean!"
exit 0
