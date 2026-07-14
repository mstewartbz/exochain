#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
  printf 'demo adjacent-boundary test failed: %s\n' "$1" >&2
  exit 1
}

[[ -f demo/LICENSE ]] || fail "demo/LICENSE is missing"
[[ -f demo/ADJACENT-SURFACE-INTAKE.md ]] || fail "demo intake record is missing"

node <<'NODE'
const fs = require('node:fs');
const path = require('node:path');

const fail = (message) => {
  console.error(`demo adjacent-boundary test failed: ${message}`);
  process.exit(1);
};

function packageFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === 'node_modules' || entry.name === 'wasm') return [];
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return packageFiles(full);
    return entry.name === 'package.json' ? [full] : [];
  });
}

for (const filename of packageFiles('demo')) {
  const pkg = JSON.parse(fs.readFileSync(filename, 'utf8'));
  if (filename === path.join('demo', 'packages', 'exochain-wasm', 'package.json')) {
    if (pkg.license !== 'Apache-2.0') fail(`${filename} must retain Apache-2.0`);
    continue;
  }
  if (pkg.private !== true) fail(`${filename} must be private`);
  if (pkg.license !== 'UNLICENSED') fail(`${filename} must be UNLICENSED`);
}

const license = fs.readFileSync('demo/LICENSE', 'utf8');
for (const phrase of [
  'proprietary',
  'EXOCHAIN bailment licensure',
  'usage accounting',
  'CrossChecked',
  'LiveSafe',
]) {
  if (!license.includes(phrase)) fail(`demo/LICENSE must state ${phrase}`);
}

for (const app of ['crosschecked', 'livesafe', 'vitallock']) {
  const intake = `demo/apps/${app}/ADJACENT-SURFACE-INTAKE.md`;
  if (!fs.existsSync(intake)) fail(`${intake} is missing`);
}
NODE

apache_files="$(git ls-files demo | xargs grep -l 'SPDX-License-Identifier: Apache-2.0' || true)"
if [[ -n "$apache_files" ]]; then
  invalid_apache_files="$(printf '%s\n' "$apache_files" | grep -v '^demo/packages/exochain-wasm/' || true)"
  [[ -z "$invalid_apache_files" ]] || {
    printf '%s\n' "$invalid_apache_files" >&2
    fail "Apache-2.0 headers remain on proprietary adjacent demo files"
  }
fi

tracked_demo_files="$(mktemp)"
trap 'rm -f "$tracked_demo_files"' EXIT
git ls-files demo | grep -Ev '(^|/)(node_modules|wasm)/' > "$tracked_demo_files"

if xargs grep -En \
  'exochain_dev|postgres://exochain:exochain|DATABASE_URL:-postgres://|POSTGRES_PASSWORD:[[:space:]]*exochain_dev' \
  < "$tracked_demo_files"; then
  fail "hardcoded demo database credentials or fallbacks remain"
fi

grep -Fq 'POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?' demo/infra/docker-compose.yml \
  || fail "compose must require POSTGRES_PASSWORD"
grep -Fq 'DATABASE_URL is required' demo/packages/shared/src/index.js \
  || fail "shared database helper must require DATABASE_URL"
grep -Fq '${DATABASE_URL:?set DATABASE_URL' demo/scripts/dev.sh \
  || fail "demo development script must require DATABASE_URL"

printf 'demo adjacent-boundary test passed\n'
