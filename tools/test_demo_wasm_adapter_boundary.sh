#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
  printf 'demo wasm-adapter test failed: %s\n' "$1" >&2
  exit 1
}

node <<'NODE'
const fs = require('node:fs');

const fail = (message) => {
  console.error(`demo wasm-adapter test failed: ${message}`);
  process.exit(1);
};

const root = JSON.parse(fs.readFileSync('demo/package.json', 'utf8'));
const wrapper = JSON.parse(fs.readFileSync('demo/packages/exochain-wasm/package.json', 'utf8'));
const testSource = fs.readFileSync('demo/packages/exochain-wasm/test.mjs', 'utf8');

if (root.scripts?.['build:wasm'] !== 'bash scripts/build-wasm.sh') {
  fail('demo build:wasm must use the repository-root-safe build script');
}
if (wrapper.license !== 'Apache-2.0') {
  fail(`core WASM wrapper must be Apache-2.0; found ${wrapper.license || '<missing>'}`);
}

for (const required of [
  'package license matches EXOCHAIN Apache-2.0 license position',
  'raw secret-key crypto entrypoints fail closed',
  'wasm_sign_with_ephemeral_key',
  'wasm_create_event_with_signature',
  'wasm_shamir_split_with_entropy',
]) {
  if (!testSource.includes(required)) fail(`WASM test is missing: ${required}`);
}

const metadataCheck = testSource.indexOf('package license matches EXOCHAIN Apache-2.0 license position');
const runtimeLoad = testSource.indexOf("require('./wasm/exochain_wasm.js')");
if (metadataCheck < 0 || runtimeLoad < 0 || metadataCheck > runtimeLoad) {
  fail('package metadata must be checked before loading generated WASM');
}
NODE

printf 'demo wasm-adapter test passed\n'
