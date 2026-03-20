#!/bin/bash
# Build ExoChain Rust crates → WASM → Node.js
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="${REPO_ROOT}/demo/packages/exochain-wasm/wasm"

echo "Building ExoChain WASM from ${REPO_ROOT}/crates/exochain-wasm..."
echo "Output: ${OUT_DIR}"

cd "${REPO_ROOT}"
wasm-pack build crates/exochain-wasm --target nodejs --release --out-dir "${OUT_DIR}"

echo ""
echo "WASM binary size: $(wc -c < "${OUT_DIR}/exochain_wasm_bg.wasm") bytes"
echo "Running tests..."
cd "${REPO_ROOT}/demo"
node packages/exochain-wasm/test.mjs
