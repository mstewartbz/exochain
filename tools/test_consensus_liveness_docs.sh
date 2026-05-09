#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

guide="docs/guides/architecture-overview.md"
consensus="crates/exo-dag/src/consensus.rs"

if grep -q "view-change on timeout" "$guide"; then
  echo "architecture guide must not claim view-change on timeout for current DAG consensus" >&2
  exit 1
fi

if grep -q "BFT-HotStuff derivative" "$guide"; then
  echo "architecture guide must not describe current DAG consensus as HotStuff-derived" >&2
  exit 1
fi

grep -q "Current liveness boundary" "$guide" || {
  echo "architecture guide must document the current consensus liveness boundary" >&2
  exit 1
}

grep -q "Liveness assumptions" "$consensus" || {
  echo "exo-dag consensus module docs must spell out liveness assumptions" >&2
  exit 1
}

grep -q "does not implement leader election or view-change" "$consensus" || {
  echo "exo-dag consensus docs must explicitly reject unstated leader/view-change behavior" >&2
  exit 1
}

echo "consensus liveness documentation test passed"
