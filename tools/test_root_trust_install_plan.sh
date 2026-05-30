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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
INSTALL_SCRIPT="$REPO_ROOT/tools/root-trust-install.sh"
SOURCE_ARTIFACT="/Users/bobstewart/exo-ceremony/bundle.json"

fail() {
  echo "root-trust-install-plan test failed: $*" >&2
  exit 1
}

expect_success() {
  local source="$1"
  local publish_root="$2"

  mkdir -p "$publish_root"

  if ! "$INSTALL_SCRIPT" --source "$source" --publish-root "$publish_root" >/tmp/root-trust-install.out.txt; then
    cat /tmp/root-trust-install.out.txt >&2
    fail "expected installation success but command failed: $source"
  fi
}

expect_failure() {
  local source="$1"
  local publish_root="$2"

  if "$INSTALL_SCRIPT" --source "$source" --publish-root "$publish_root" >/tmp/root-trust-install.out.txt; then
    fail "expected installation failure but command succeeded"
  fi
}

bundle_id_hex() {
  local file="$1"
  python3 - "$file" <<'PY'
import json
import sys
from pathlib import Path

with Path(sys.argv[1]).open('r', encoding='utf-8') as handle:
    artifact = json.load(handle)
bundle_id = artifact['bundle_id']
print(''.join(f'{value:02x}' for value in bundle_id))
PY
}

latest_record_id() {
  local manifest_path="$1"
  python3 - "$manifest_path" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
print(manifest['latest_record_id'])
PY
}

consumer_must_fail_if_missing_bundle() {
  local publish_root="$1"
  local record_id
  local pointer_path

  record_id="$(latest_record_id "$publish_root/install-manifest.json")"
  pointer_path="$publish_root/root-trust-pointer.${record_id}.json"

  if python3 - "$pointer_path" "$publish_root/root-trust-bundle.canonical.json" <<'PY'
import sys
import json
from pathlib import Path
from blake3 import blake3

pointer = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
bundle_path = Path(sys.argv[2])

if pointer.get('verification_status') != 'verified':
    raise SystemExit('pointer verification_status != verified')

expected_pointer = pointer.get('pointer_checksum', {}).get('value')
if not expected_pointer:
    raise SystemExit('pointer checksum missing')
payload = dict(pointer)
payload.pop('pointer_checksum', None)
actual_pointer = blake3(json.dumps(payload, sort_keys=True, separators=(',', ':')).encode('utf-8')).hexdigest()
if expected_pointer != actual_pointer:
    raise SystemExit('pointer checksum mismatch')

if not bundle_path.exists():
    raise SystemExit('published bundle missing')

expected_bundle = pointer.get('bundle_checksum', {}).get('value')
actual_bundle = blake3(bundle_path.read_bytes()).hexdigest()
if expected_bundle != actual_bundle:
    raise SystemExit('bundle checksum mismatch')
PY
  then
    fail "consumer gate should fail closed when bundle is missing"
  fi
}

tmp_root="$(mktemp -d -t exo-root-trust-plan.XXXXXX)"

# 1) Happy-path install validation
install_root="$tmp_root/happy"
expect_success "$SOURCE_ARTIFACT" "$install_root"
[ -f "$install_root/root-trust-bundle.canonical.json" ] || fail "canonical artifact missing"
record_id="$(latest_record_id "$install_root/install-manifest.json")"
[ -f "$install_root/root-trust-pointer.${record_id}.json" ] || fail "pointer missing"

bundle_hex="$(bundle_id_hex "$install_root/root-trust-bundle.canonical.json")"
expected_hex="$(bundle_id_hex "$SOURCE_ARTIFACT")"
[ "$bundle_hex" = "$expected_hex" ] || fail "bundle-id mismatch"

# 2) Missing field failure
missing_field_artifact="$tmp_root/missing-field.json"
python3 - "$SOURCE_ARTIFACT" "$missing_field_artifact" <<'PY'
import json
import sys
from pathlib import Path

artifact = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
del artifact['transcript_hash']
Path(sys.argv[2]).write_text(json.dumps(artifact), encoding='utf-8')
PY
expect_failure "$missing_field_artifact" "$tmp_root/fail-missing-field"

# 3) Signature tamper failure
signature_tamper_artifact="$tmp_root/signature-tamper.json"
python3 - "$SOURCE_ARTIFACT" "$signature_tamper_artifact" <<'PY'
import json
import sys
from pathlib import Path

artifact = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
signature = artifact['root_signature']['signature']
signature[0] = (signature[0] + 1) % 256
artifact['root_signature']['signature'] = signature
Path(sys.argv[2]).write_text(json.dumps(artifact), encoding='utf-8')
PY
expect_failure "$signature_tamper_artifact" "$tmp_root/fail-signature"

# 4) Identity/certificate tamper failure
identity_tamper_artifact="$tmp_root/identity-tamper.json"
python3 - "$SOURCE_ARTIFACT" "$identity_tamper_artifact" <<'PY'
import json
import sys
from pathlib import Path

artifact = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
artifact['root_signature']['signer_ids'] = [1, 2, 3, 4, 5, 6]
artifact['config']['threshold'] = 6
artifact['config']['ceremony_id'] = 'tampered'
Path(sys.argv[2]).write_text(json.dumps(artifact), encoding='utf-8')
PY
expect_failure "$identity_tamper_artifact" "$tmp_root/fail-identity"

# 5) Deployment fail-closed: missing artifact at pointer URI
fail_closed_root="$tmp_root/fail-closed"
expect_success "$SOURCE_ARTIFACT" "$fail_closed_root"
rm -f "$fail_closed_root/root-trust-bundle.canonical.json"
consumer_must_fail_if_missing_bundle "$fail_closed_root"

echo "root-trust-install-plan test matrix passed"
