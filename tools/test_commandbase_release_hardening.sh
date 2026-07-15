#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

fail() {
  printf 'commandbase release-hardening test failed: %s\n' "$1" >&2
  exit 1
}

manifest="command-base/app/package.json"
lockfile="command-base/app/package-lock.json"

[[ -f "$manifest" ]] || fail "$manifest is missing"
[[ -f "$lockfile" ]] || fail "$lockfile is missing"
[[ -f command-base/app/lib/upload-policy.js ]] || fail "upload allowlist policy is missing"
[[ -f command-base/app/lib/webhook-auth.js ]] || fail "webhook authentication boundary is missing"
[[ -f command-base/LICENSE ]] || fail "command-base/LICENSE is missing"
grep -F 'Proprietary and Confidential' command-base/LICENSE >/dev/null \
  || fail 'command-base/LICENSE must declare the proprietary boundary'
grep -F 'No license, express or implied, is granted' command-base/LICENSE >/dev/null \
  || fail 'command-base/LICENSE must deny implicit grants'

node <<'NODE'
const fs = require('node:fs');

const fail = (message) => {
  console.error(`commandbase release-hardening test failed: ${message}`);
  process.exit(1);
};

const pkg = JSON.parse(fs.readFileSync('command-base/app/package.json', 'utf8'));
const workerPkg = JSON.parse(fs.readFileSync('command-base/worker/package.json', 'utf8'));
const lock = JSON.parse(fs.readFileSync('command-base/app/package-lock.json', 'utf8'));
const scripts = pkg.scripts || {};
const dependencies = pkg.dependencies || {};

if (pkg.license !== 'UNLICENSED') {
  fail('command-base/app/package.json must declare UNLICENSED');
}
if (workerPkg.license !== 'UNLICENSED') {
  fail('command-base/worker/package.json must declare UNLICENSED');
}
if (!lock.packages || !lock.packages[''] || lock.packages[''].license !== 'UNLICENSED') {
  fail('command-base/app/package-lock.json root package must declare UNLICENSED');
}

if (scripts.preinstall !== 'npm audit --audit-level=critical') {
  fail('preinstall must fail closed with npm audit --audit-level=critical');
}
if (Object.values(scripts).some((script) => /npm audit[^\n]*(?:\|\|\s*true|;\s*true)/.test(script))) {
  fail('npm audit scripts must not suppress failures');
}
if (dependencies.multer !== '2.2.0') {
  fail(`multer must be exactly 2.2.0; found ${dependencies.multer || '<missing>'}`);
}

const root = lock.packages && lock.packages[''];
const locked = lock.packages && lock.packages['node_modules/multer'];
if (!root || !root.dependencies || root.dependencies.multer !== '2.2.0') {
  fail('package-lock root must pin multer 2.2.0');
}
if (!locked || locked.version !== '2.2.0') {
  fail(`package-lock must resolve multer 2.2.0; found ${locked ? locked.version : '<missing>'}`);
}
NODE

apache_spdx='SPDX-License-Identifier: Apache-''2.0'
apache_grant='Licensed under the Apache'' License'
if git grep -n -e "$apache_spdx" -e "$apache_grant" -- command-base; then
  fail 'CommandBase retains explicit Apache-2.0 file grants'
fi

grep -F '"command-base/",' tools/license_headers.py >/dev/null \
  || fail 'Apache header utility must exclude command-base/'
grep -F 'proprietary adjacent surface' command-base/EXOCHAIN_SURFACE_INTAKE.md >/dev/null \
  || fail 'CommandBase intake must classify the subtree as proprietary adjacent code'

auth_source="$(<command-base/app/lib/auth.js)"
[[ "$auth_source" == *"EXOCHAIN_AUTH_SECRET"* ]] || fail "auth fallback must use EXOCHAIN_AUTH_SECRET"
[[ "$auth_source" == *"MIN_HMAC_SECRET_BYTES = 32"* ]] || fail "auth fallback must require at least 32 bytes"
[[ "$auth_source" != *"exochain-dev-secret-change-in-production"* ]] || fail "historical development auth secret remains in production source"

webhook_source="$(<command-base/app/lib/webhook-auth.js)"
[[ "$webhook_source" == *"COMMANDBASE_WEBHOOK_SECRET"* ]] || fail "webhook secret must come from COMMANDBASE_WEBHOOK_SECRET"
[[ "$webhook_source" == *"MIN_WEBHOOK_SECRET_BYTES = 32"* ]] || fail "webhook secret must require at least 32 bytes"
[[ "$webhook_source" == *"timingSafeEqual"* ]] || fail "webhook comparison must be timing safe"

server_source="$(<command-base/app/server.js)"
[[ "$server_source" != *"req.query.secret"* ]] || fail "webhook secrets must not be accepted in query parameters"
[[ "$server_source" == *"commandBaseUploadFileFilter"* ]] || fail "server upload paths must use the shared allowlist"

settings_source="$(<command-base/app/routes/settings.js)"
[[ "$settings_source" == *"looksMaskedSecretValue"* ]] || fail "provider writes must reject masked sentinels"
[[ "$settings_source" != *"value: row.encrypted_value"* ]] || fail "credential read route returns a raw secret"

required_tests=(
  command-base/app/routes/refinement.test.js
  command-base/app/services/cqi-orchestrator.test.js
  command-base/app/server-assignment-determinism.test.js
  command-base/app/server-autofill-determinism.test.js
  command-base/app/provider-key-boundary.test.js
  command-base/app/lib/upload-policy.test.js
  command-base/app/lib/auth.test.js
  command-base/app/lib/webhook-auth.test.js
  command-base/app/server-webhook-secret-boundary.test.js
  command-base/app/lib/task-force-engine.test.js
)

for test_file in "${required_tests[@]}"; do
  [[ -f "$test_file" ]] || fail "$test_file is missing"
done

printf 'commandbase release-hardening test passed\n'
