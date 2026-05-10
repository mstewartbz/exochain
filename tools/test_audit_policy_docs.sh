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

cd "$(dirname "$0")/.."

current_policy_files=(
  ".github/workflows/ci.yml"
  ".github/workflows/release.yml"
  "deny.toml"
  ".cargo/audit.toml"
  "README.md"
  "SECURITY.md"
  "VERSIONING.md"
  "INTEGRATION.md"
  "docs/architecture/THREAT-MODEL.md"
  "docs/audit/REVIEW-2026-04-19.md"
  "docs/grant/CODEX-CYBERSECURITY-GRANT-CLAIMS.md"
)

for file in "${current_policy_files[@]}"; do
  if grep -q "cargo audit --deny warnings" "$file"; then
    echo "$file must not claim Gate 6 runs cargo audit --deny warnings" >&2
    exit 1
  fi

  if grep -Eq "audit.*--deny warnings|--deny warnings.*audit" "$file"; then
    echo "$file must not describe the audit gate as --deny warnings" >&2
    exit 1
  fi

  if grep -q 'yanked = "deny"' "$file"; then
    echo "$file must not claim yanked advisories are denied by the current audit gate" >&2
    exit 1
  fi

  if grep -q "rejecting any dependency with a known security advisory" "$file"; then
    echo "$file must not overstate advisory rejection while documented ignores exist" >&2
    exit 1
  fi

  if grep -q "Zero dependency vulnerabilities" "$file"; then
    echo "$file must not claim zero dependency vulnerabilities while advisory ignores exist" >&2
    exit 1
  fi

  if grep -q "no advisories exist" "$file"; then
    echo "$file must say policy-enforced with documented advisory exceptions, not no advisories exist" >&2
    exit 1
  fi

  if grep -q "no known vulnerabilities" "$file"; then
    echo "$file must say policy-enforced with documented advisory exceptions, not no known vulnerabilities" >&2
    exit 1
  fi

  if [ "$file" != ".github/workflows/release.yml" ] && grep -q "provenance.json" "$file"; then
    echo "$file must describe CycloneDX SBOM and SLSA attestations, not provenance.json" >&2
    exit 1
  fi

  if grep -q "every release tag is cryptographically signed" "$file"; then
    echo "$file must distinguish current unsigned pre-release tags from formal signed-release policy" >&2
    exit 1
  fi
done

grep -q "cargo audit --deny unsound --deny unmaintained" .github/workflows/ci.yml || {
  echo "CI must expose the current cargo-audit deny flags" >&2
  exit 1
}

grep -q "vulnerabilities by default plus unsound and unmaintained warnings" deny.toml || {
  echo "deny.toml must describe the current cargo-audit deny scope" >&2
  exit 1
}

grep -q "Yanked advisories are warning-only" .cargo/audit.toml || {
  echo ".cargo/audit.toml must document the current yanked-advisory posture" >&2
  exit 1
}

grep -q "yanked advisories currently remain warnings" docs/architecture/THREAT-MODEL.md || {
  echo "threat model must document the current yanked-advisory posture" >&2
  exit 1
}

echo "audit policy documentation test passed"
