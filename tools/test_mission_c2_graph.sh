#!/usr/bin/env bash
# Copyright 2026 Exochain Foundation
# SPDX-License-Identifier: Apache-2.0
# Guard: every mission-graph.yaml node has a matching docs/c2/nodes/<id>.md

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
YAML="$ROOT/docs/c2/mission-graph.yaml"
NODES_DIR="$ROOT/docs/c2/nodes"

if [[ ! -f "$YAML" ]]; then
  echo "FAIL: missing $YAML" >&2
  exit 1
fi

missing=0
while IFS= read -r id; do
  [[ -z "$id" ]] && continue
  page="$NODES_DIR/${id}.md"
  if [[ ! -f "$page" ]]; then
    echo "FAIL: node '$id' missing page $page" >&2
    missing=1
  fi
done < <(grep -E '^\s+- id:' "$YAML" | sed -E 's/.*id:[[:space:]]*//')

if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

echo "OK: mission-graph.yaml nodes have matching docs/c2/nodes/*.md pages"
