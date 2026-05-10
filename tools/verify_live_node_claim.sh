#!/usr/bin/env bash
set -euo pipefail

url="${1:-}"
if [ -z "$url" ]; then
  echo "usage: $0 https://node.example" >&2
  exit 2
fi

body=$(curl -fsS --max-time 10 "$url/health")
printf '%s\n' "$body" | jq -e '.status == "ok" or .status == "healthy"' >/dev/null

if printf '%s\n' "$body" | rg -i 'token|secret|private|password|api[_-]?key|seed|mnemonic'; then
  echo "live node health response leaks sensitive-looking fields" >&2
  exit 1
fi

printf 'live node health verified: %s/health\n' "$url"
