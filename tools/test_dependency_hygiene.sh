#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

out=$(mktemp)
trap 'rm -f "$out"' EXIT

cargo deny check >"$out" 2>&1
duplicates=$(rg -c '^warning\[duplicate\]' "$out" || true)

max_duplicates=24
if [ "$duplicates" -gt "$max_duplicates" ]; then
  cat "$out" >&2
  echo "duplicate dependency warning count $duplicates exceeds cap $max_duplicates" >&2
  exit 1
fi

echo "dependency hygiene check passed: $duplicates duplicate warnings (cap $max_duplicates)"
