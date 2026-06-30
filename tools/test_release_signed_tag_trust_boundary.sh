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

fail() {
  printf 'release signed-tag trust boundary test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/release.yml"
ci_workflow=".github/workflows/ci.yml"

[[ -f "$workflow" ]] || fail "$workflow is missing"
[[ -f "$ci_workflow" ]] || fail "$ci_workflow is missing"

verify_block=$(
  awk '
    $0 == "  verify-signed-tag:" { capture = 1; print; next }
    capture && $0 ~ /^  [A-Za-z0-9_-]+:$/ { exit }
    capture { print }
  ' "$workflow"
)
[[ -n "$verify_block" ]] || fail "verify-signed-tag job is missing"

grep -F 'EXOCHAIN_RELEASE_SIGNING_PUBLIC_KEY_ASC: ${{ vars.EXOCHAIN_RELEASE_SIGNING_PUBLIC_KEY_ASC }}' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must receive the approved release public key from repository variables"
grep -F 'EXOCHAIN_RELEASE_SIGNING_FINGERPRINT: ${{ vars.EXOCHAIN_RELEASE_SIGNING_FINGERPRINT }}' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must receive the approved release signing fingerprint from repository variables"
grep -F 'GNUPGHOME="$(mktemp -d)"' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must isolate the release verification keyring"
grep -F 'gpg --batch --import' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must import the approved release public key before verification"
grep -F '^[0-9A-F]{40}$' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must enforce a full 40-hex-character OpenPGP fingerprint"
grep -F 'imported_fingerprint="$(gpg --batch --with-colons --fingerprint "$signing_key_fingerprint"' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must prove the imported key matches the configured fingerprint"
grep -F 'git tag -v "${RELEASE_TAG}"' <<<"$verify_block" >/dev/null \
  || fail "verify-signed-tag must still use git tag -v for the release tag"

grep -F 'bash tools/test_release_signed_tag_trust_boundary.sh' "$ci_workflow" >/dev/null \
  || fail "CI repo hygiene must run the release signed-tag trust boundary guard"

printf 'release signed-tag trust boundary test passed\n'
