#!/usr/bin/env bash
set -euo pipefail

: "${OPENAI_API_KEY:?Set OPENAI_API_KEY}"
: "${ANTHROPIC_API_KEY:?Set ANTHROPIC_API_KEY}"
: "${PERPLEXITY_API_KEY:?Set PERPLEXITY_API_KEY}"

CFG=${1:-config.yaml}
export TMPDIR="${TMPDIR:-$PWD/.tmp}"
mkdir -p "$TMPDIR"

engines=(openai-latest anthropic-opus-4-1 perplexity-sonar)

for e in "${engines[@]}"; do
  echo "== Testing $e =="
  cargo run -- -c "$CFG" engine test "$e" || true
  echo
done
