#!/usr/bin/env bash
# Generate random 4-digit number, SHA256 hash it, output JSON
NUM=$(( RANDOM % 9000 + 1000 ))
HASH=$(echo -n "$NUM" | sha256sum | awk '{print $1}')
echo "{\"num\": $NUM, \"hash\": \"$HASH\"}"
