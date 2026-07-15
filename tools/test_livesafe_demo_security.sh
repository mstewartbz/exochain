#!/usr/bin/env bash
# Copyright 2026 Exochain Foundation
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at:
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_dir="$repo_root/demo/apps/livesafe"
vite_config="$app_dir/vite.config.ts"
package_json="$app_dir/package.json"
package_lock="$app_dir/package-lock.json"
license_file="$app_dir/LICENSE"
intake_file="$app_dir/INTAKE.md"

[ -f "$license_file" ] || {
  echo "LiveSafe demo proprietary LICENSE is required" >&2
  exit 1
}
grep -F 'Proprietary and Confidential' "$license_file" >/dev/null
grep -F 'No license, express or implied, is granted' "$license_file" >/dev/null

[ -f "$intake_file" ] || {
  echo "LiveSafe demo adjacent-surface intake is required" >&2
  exit 1
}
for intake_boundary in \
  'Accountable maintainer:' \
  'Deployment status:' \
  'not allowed to claim EXOCHAIN constitutional enforcement' \
  'Direct EXOCHAIN core reads: none' \
  'Direct EXOCHAIN core writes: none' \
  'Boundary guard:' \
  'Runtime source:' \
  'Disablement:'; do
  grep -F "$intake_boundary" "$intake_file" >/dev/null || {
    echo "LiveSafe demo intake missing boundary: $intake_boundary" >&2
    exit 1
  }
done

if grep -Eq 'allowedHosts:[[:space:]]*true' "$vite_config"; then
  echo "LiveSafe demo Vite preview must not disable Host header checks" >&2
  exit 1
fi

if git -C "$repo_root" grep -n -F 'Keys are derived locally' -- demo/apps/livesafe; then
  echo "LiveSafe demo must not claim API-generated keys are derived locally" >&2
  exit 1
fi

grep -F 'LiveSafe encryption keys are returned by the adjacent API' \
  "$app_dir/src/pages/Login.tsx" >/dev/null || {
  echo "LiveSafe demo login must disclose adjacent API key provenance" >&2
  exit 1
}

if ! grep -Eq 'allowedHosts:[[:space:]]*previewAllowedHosts' "$vite_config"; then
  echo "LiveSafe demo Vite preview must use the explicit previewAllowedHosts allowlist" >&2
  exit 1
fi

node - "$package_json" "$package_lock" <<'NODE'
const fs = require('node:fs');
const [packagePath, lockPath] = process.argv.slice(2);
const manifest = JSON.parse(fs.readFileSync(packagePath, 'utf8'));

if (!fs.existsSync(lockPath)) {
  throw new Error('LiveSafe demo package-lock.json is required for deploy reproducibility');
}

const lock = JSON.parse(fs.readFileSync(lockPath, 'utf8'));
const rootLock = lock.packages?.[''];

if (!rootLock) {
  throw new Error('LiveSafe demo package-lock.json must include root package metadata');
}

if (manifest.private !== true || manifest.license !== 'UNLICENSED') {
  throw new Error('LiveSafe demo manifest must remain private and UNLICENSED');
}

if (rootLock.license !== 'UNLICENSED') {
  throw new Error('LiveSafe demo package-lock root must remain UNLICENSED');
}

for (const section of ['dependencies', 'devDependencies']) {
  const declared = manifest[section] || {};
  const locked = rootLock[section] || {};

  for (const [name, version] of Object.entries(declared)) {
    if (/^[~^*<>]/u.test(version)) {
      throw new Error(`${section}.${name} must be pinned exactly, got ${version}`);
    }

    if (locked[name] !== version) {
      throw new Error(`${section}.${name} package-lock mismatch: ${locked[name]} != ${version}`);
    }
  }
}
NODE

apache_spdx='SPDX-License-Identifier: Apache-''2.0'
apache_grant='Licensed under the Apache'' License'
if git -C "$repo_root" grep -n -e "$apache_spdx" -e "$apache_grant" -- demo/apps/livesafe; then
  echo "LiveSafe demo files must not retain Apache-2.0 grants" >&2
  exit 1
fi

grep -F '"demo/apps/livesafe/",' "$repo_root/tools/license_headers.py" >/dev/null || {
  echo "Apache header utility must exclude the LiveSafe demo" >&2
  exit 1
}

echo "LiveSafe demo security guard passed"
