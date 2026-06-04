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

assert_verifier_policy_recorded() {
  local source="$1"
  local publish_root="$2"

  python3 - "$source" "$publish_root/install-manifest.json" "$REPO_ROOT" <<'PY'
import json
import subprocess
import sys
from pathlib import Path

source = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
manifest = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
repo_root = sys.argv[3]
trusted_head = subprocess.check_output(
    ["git", "-C", repo_root, "rev-parse", "HEAD"],
    text=True,
).strip()
source_bundle_repo_commit = source["config"]["repo_commit"]

record = manifest["records"][-1]
verification = record["verification"]
policy = record["policy"]

if verification["trusted_verifier_commit"] != trusted_head:
    raise SystemExit("trusted verifier commit was not current HEAD")
if verification["source_bundle_repo_commit"] != source_bundle_repo_commit:
    raise SystemExit("source bundle repo commit was not preserved")
if policy["verifier_commit_authority"] != "operator trusted policy, never imported bundle contents":
    raise SystemExit("verifier commit authority policy missing")

pointer_path = Path(sys.argv[2]).parent / verification["pointer_file"]
pointer = json.loads(pointer_path.read_text(encoding="utf-8"))
if pointer["trusted_verifier_commit"] != trusted_head:
    raise SystemExit("pointer trusted verifier commit mismatch")
if pointer["source_bundle_repo_commit"] != source_bundle_repo_commit:
    raise SystemExit("pointer source bundle repo commit mismatch")
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

# 0) Verifier executable policy guard
python3 - "$INSTALL_SCRIPT" <<'PY'
import sys
from pathlib import Path

script = Path(sys.argv[1]).read_text(encoding="utf-8")

required_fragments = [
    "--trusted-verifier-commit",
    "EXO_ROOT_TRUST_VERIFIER_COMMIT",
    "trusted_verifier_commit",
    "source_bundle_repo_commit",
]
missing = [fragment for fragment in required_fragments if fragment not in script]
if missing:
    raise SystemExit(
        "installer is missing trusted verifier policy fragments: "
        + ", ".join(missing)
    )

forbidden_fragments = [
    '"verifier_commit": repo_commit',
    '["verifier_commit"])',
    "verifier_commit = metadata[\"verifier_commit\"]",
]
present = [fragment for fragment in forbidden_fragments if fragment in script]
if present:
    raise SystemExit(
        "installer still derives executable verifier commit from imported bundle metadata: "
        + ", ".join(present)
    )
PY

# Invalid verifier policy must fail before install or publication.
if EXO_ROOT_TRUST_VERIFIER_COMMIT=not-a-commit \
  "$INSTALL_SCRIPT" --source "$SOURCE_ARTIFACT" --publish-root "$tmp_root/fail-verifier-policy" \
  >/tmp/root-trust-install.out.txt 2>/tmp/root-trust-install.err.txt; then
  fail "expected invalid trusted verifier commit to fail closed"
fi

# 1) Happy-path install validation
install_root="$tmp_root/happy"
expect_success "$SOURCE_ARTIFACT" "$install_root"
[ -f "$install_root/root-trust-bundle.canonical.json" ] || fail "canonical artifact missing"
record_id="$(latest_record_id "$install_root/install-manifest.json")"
[ -f "$install_root/root-trust-pointer.${record_id}.json" ] || fail "pointer missing"

bundle_hex="$(bundle_id_hex "$install_root/root-trust-bundle.canonical.json")"
expected_hex="$(bundle_id_hex "$SOURCE_ARTIFACT")"
[ "$bundle_hex" = "$expected_hex" ] || fail "bundle-id mismatch"
assert_verifier_policy_recorded "$SOURCE_ARTIFACT" "$install_root"

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
