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

ARTIFACT_ID="avc-exo-ceremony-2026"
DEFAULT_SOURCE="/Users/bobstewart/exo-ceremony/bundle.json"
DEFAULT_PUBLISH_ROOT="$REPO_ROOT/artifacts/trust/$ARTIFACT_ID"

fail() {
  echo "[root-trust-install] $*" >&2
  exit 1
}

usage() {
  cat <<'EOF_USAGE'
Usage: tools/root-trust-install.sh [options]

Install and publish a ceremony-root trust bundle only after strict exo-node
verification by a trusted operator-selected verifier commit.

Options:
  --source <path>      Source ceremony artifact path
                       (default: /Users/bobstewart/exo-ceremony/bundle.json)
  --artifact-id <id>   Artifact identifier
                       (default: avc-exo-ceremony-2026)
  --publish-root <dir> Directory for immutable published artifact
                       (default: <repo>/artifacts/trust/<artifact-id>)
  --trusted-verifier-commit <commit>
                       Trusted 40-character verifier commit to execute.
                       Defaults to EXO_ROOT_TRUST_VERIFIER_COMMIT when set,
                       otherwise the current repository HEAD.
  --help               Show this help text

This adjacent-surface process:
- treats the source bundle as imported evidence
- preserves the bundle exactly as emitted by assemble-bundle
- verifies with an operator-trusted exo-node commit, not with code selected by
  imported bundle contents
- records the bundle's config.repo_commit only as signed source-bundle data
- publishes a read-only canonical artifact only on verification success
- writes append-only installation metadata and pointer records
EOF_USAGE
}

source_path="$DEFAULT_SOURCE"
publish_root="$DEFAULT_PUBLISH_ROOT"
artifact_id="$ARTIFACT_ID"
publish_root_set=0
trusted_verifier_commit="${EXO_ROOT_TRUST_VERIFIER_COMMIT:-}"
trusted_verifier_commit_source="current repository HEAD"
if [[ -n "$trusted_verifier_commit" ]]; then
  trusted_verifier_commit_source="EXO_ROOT_TRUST_VERIFIER_COMMIT"
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)
      source_path="$2"
      shift 2
      ;;
    --artifact-id)
      artifact_id="$2"
      if [[ "$publish_root_set" -eq 0 ]]; then
        publish_root="$REPO_ROOT/artifacts/trust/$artifact_id"
      fi
      shift 2
      ;;
    --publish-root)
      publish_root="$2"
      publish_root_set=1
      shift 2
      ;;
    --trusted-verifier-commit)
      trusted_verifier_commit="$2"
      trusted_verifier_commit_source="--trusted-verifier-commit"
      shift 2
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      fail "Unknown argument: $1"
      ;;
  esac
done

[[ -n "$source_path" ]] || fail "Missing --source value"
[[ -f "$source_path" ]] || fail "Source artifact missing: $source_path"
command -v cargo >/dev/null 2>&1 || fail "Required command missing: cargo"
command -v git >/dev/null 2>&1 || fail "Required command missing: git"
command -v python3 >/dev/null 2>&1 || fail "Required command missing: python3"

if [[ -z "$trusted_verifier_commit" ]]; then
  trusted_verifier_commit="$(git -C "$REPO_ROOT" rev-parse HEAD)" \
    || fail "Failed to resolve trusted verifier commit from current HEAD"
fi
if [[ ! "$trusted_verifier_commit" =~ ^[0-9a-f]{40}$ ]]; then
  fail "trusted verifier commit must be a 40-character lowercase hex commit"
fi

if ! python3 - <<'PY_CHECK'
import importlib.util
if importlib.util.find_spec("blake3") is None:
    raise SystemExit(1)
PY_CHECK
then
  fail "Required python dependency missing: blake3"
fi

mkdir -p "$publish_root" || fail "Failed to create publish root: $publish_root"

tmp_dir="$(mktemp -d -t exo-root-trust-install.XXXXXX)"
trap 'rm -rf "$tmp_dir"' EXIT

canonical_bundle="$tmp_dir/${artifact_id}.canonical.json"
verify_input="$tmp_dir/${artifact_id}.verify.input.json"
metadata_path="$tmp_dir/${artifact_id}.metadata.json"
verification_output="$tmp_dir/verify.output.json"
verification_error="$tmp_dir/verify.error.log"
verifier_source="$tmp_dir/verifier-source"
publish_bundle="$publish_root/root-trust-bundle.canonical.json"
manifest_path="$publish_root/install-manifest.json"

python3 - "$source_path" "$canonical_bundle" "$verify_input" "$metadata_path" "$artifact_id" "$trusted_verifier_commit" "$trusted_verifier_commit_source" <<'PY'
import json
import re
import sys
from pathlib import Path
from blake3 import blake3

source_path = Path(sys.argv[1])
canonical_bundle_path = Path(sys.argv[2])
verify_input_path = Path(sys.argv[3])
metadata_path = Path(sys.argv[4])
artifact_id = sys.argv[5]
trusted_verifier_commit = sys.argv[6]
trusted_verifier_commit_source = sys.argv[7]

source_bundle = json.loads(source_path.read_text(encoding="utf-8"))

required_fields = [
    "config",
    "public_key_package",
    "issuer_delegation",
    "transcript_hash",
    "root_signature",
    "bundle_id",
]
for field in required_fields:
    if field not in source_bundle:
        raise SystemExit(f"missing field: {field}")

config = source_bundle["config"]
for key in ("ceremony_id", "network_id", "repo_commit", "threshold", "max_signers"):
    if key not in config:
        raise SystemExit(f"missing config.{key}")

repo_commit = config["repo_commit"]
if not isinstance(repo_commit, str) or not re.fullmatch(r"[0-9a-f]{40}", repo_commit):
    raise SystemExit("config.repo_commit must be a 40-character lowercase hex commit")
source_bundle_repo_commit = repo_commit

if not re.fullmatch(r"[0-9a-f]{40}", trusted_verifier_commit):
    raise SystemExit("trusted verifier commit must be a 40-character lowercase hex commit")

if config["threshold"] != 7 or config["max_signers"] != 13:
    raise SystemExit("expected 7-of-13 configuration")

bundle_id = source_bundle["bundle_id"]
if not isinstance(bundle_id, list) or len(bundle_id) != 32:
    raise SystemExit("bundle_id must be a 32-byte array")
if not all(isinstance(value, int) and 0 <= value <= 255 for value in bundle_id):
    raise SystemExit("bundle_id entries must be bytes")

root_signature = source_bundle["root_signature"]
if not isinstance(root_signature, dict):
    raise SystemExit("root_signature must be the emitted ceremony object")

signature_values = root_signature.get("signature")
if not isinstance(signature_values, list) or not signature_values:
    raise SystemExit("root_signature.signature must be a non-empty byte array")
if not all(isinstance(value, int) and 0 <= value <= 255 for value in signature_values):
    raise SystemExit("root_signature.signature must be byte values")

signer_ids = root_signature.get("signer_ids")
if signer_ids != [1, 2, 3, 4, 5, 6, 7]:
    raise SystemExit("root_signature.signer_ids must be [1,2,3,4,5,6,7]")

canonical_json = json.dumps(source_bundle, sort_keys=True, indent=2)
canonical_compact = json.dumps(source_bundle, sort_keys=True, separators=(",", ":"))

canonical_bundle_path.write_text(canonical_json + "\n", encoding="utf-8")
verify_input_path.write_text(
    json.dumps({"bundle": source_bundle}, separators=(",", ":")) + "\n",
    encoding="utf-8",
)

metadata = {
    "schema": "exo.root_trust_install_metadata.v1",
    "artifact_id": artifact_id,
    "source_path": str(source_path),
    "bundle_format": "emitted_root_signature_object",
    "source_bundle_repo_commit": source_bundle_repo_commit,
    "trusted_verifier_commit": trusted_verifier_commit,
    "trusted_verifier_commit_source": trusted_verifier_commit_source,
    "verifier_commit": trusted_verifier_commit,
    "source_checksum": {
        "algorithm": "BLAKE3",
        "value": blake3(source_path.read_bytes()).hexdigest(),
    },
    "source_sha256": __import__("hashlib").sha256(source_path.read_bytes()).hexdigest(),
    "canonical_bundle_checksum": {
        "algorithm": "BLAKE3",
        "value": blake3(canonical_compact.encode("utf-8")).hexdigest(),
    },
    "source_bundle": {
        "bundle_id": bundle_id,
        "bundle_id_hex": "".join(f"{value:02x}" for value in bundle_id),
        "ceremony_id": config["ceremony_id"],
        "network_id": config["network_id"],
        "repo_commit": source_bundle_repo_commit,
        "constitution_hash": config.get("constitution_hash"),
        "threshold": config["threshold"],
        "max_signers": config["max_signers"],
        "created_at": config.get("created_at"),
    },
    "root_signature_signers": signer_ids,
}

metadata_path.write_text(json.dumps(metadata, sort_keys=True, indent=2) + "\n", encoding="utf-8")
PY

verifier_commit="$(python3 - "$metadata_path" <<'PY'
import json
import sys
from pathlib import Path
print(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["trusted_verifier_commit"])
PY
)"
source_bundle_repo_commit="$(python3 - "$metadata_path" <<'PY'
import json
import sys
from pathlib import Path
print(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["source_bundle_repo_commit"])
PY
)"
verifier_short="${verifier_commit:0:12}"

if ! git -C "$REPO_ROOT" cat-file -e "$verifier_commit^{commit}" 2>/dev/null; then
  git -C "$REPO_ROOT" fetch origin "$verifier_commit" >/dev/null 2>&1 || true
fi
git -C "$REPO_ROOT" cat-file -e "$verifier_commit^{commit}" 2>/dev/null \
  || fail "Verifier commit unavailable locally or from origin: $verifier_commit"

mkdir -p "$verifier_source"
git -C "$REPO_ROOT" archive "$verifier_commit" | tar -x -C "$verifier_source"

verification_command="CARGO_TARGET_DIR=$REPO_ROOT/target-root-trust-$verifier_short cargo run -p exochain-node -- genesis verify-bundle --input $verify_input"
if ! (
  cd "$verifier_source" &&
  CARGO_TARGET_DIR="$REPO_ROOT/target-root-trust-$verifier_short" \
    cargo run -p exochain-node -- genesis verify-bundle --input "$verify_input" \
      >"$verification_output" 2>"$verification_error"
); then
  cat "$verification_error" || true
  cat "$verification_output" || true
  fail "Bundle verification command failed"
fi

if ! python3 - "$verification_output" <<'PY_VERIFY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8") as payload_handle:
    payload = json.load(payload_handle)
if payload.get("verified") is not True:
    raise SystemExit("verification result was not true")
PY_VERIFY
then
  cat "$verification_output" || true
  fail "Verification output was not a success record"
fi

file_blake3() {
  python3 - "$1" <<'PY_HASH'
from blake3 import blake3
from pathlib import Path
import sys
print(blake3(Path(sys.argv[1]).read_bytes()).hexdigest())
PY_HASH
}

if [ -f "$publish_bundle" ]; then
  existing_checksum="$(file_blake3 "$publish_bundle")"
  new_checksum="$(file_blake3 "$canonical_bundle")"
  if [[ "$existing_checksum" != "$new_checksum" ]]; then
    fail "Publish target exists with different content: $publish_bundle"
  fi
else
  cp "$canonical_bundle" "$publish_bundle"
fi
chmod 444 "$publish_bundle"

install_timestamp="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
cargo_version="$(cd "$verifier_source" && cargo --version)"

if [ -f "$manifest_path" ]; then
  chmod u+w "$manifest_path"
fi

record_id="$(python3 - "$metadata_path" "$publish_bundle" "$publish_root" "$manifest_path" "$verification_command" "$install_timestamp" "$verifier_commit" "$cargo_version" <<'PY_RECORD'
import json
import sys
from pathlib import Path
from blake3 import blake3

metadata_path = Path(sys.argv[1])
publish_bundle_path = Path(sys.argv[2])
publish_root = Path(sys.argv[3])
manifest_path = Path(sys.argv[4])
verification_command = sys.argv[5]
install_timestamp = sys.argv[6]
verifier_commit = sys.argv[7]
cargo_version = sys.argv[8]

metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
bundle_bytes = publish_bundle_path.read_bytes()
bundle_digest = blake3(bundle_bytes).hexdigest()
bundle_uri = publish_bundle_path.resolve().as_uri()

record_seed = {
    "artifact_id": metadata["artifact_id"],
    "verified_at": install_timestamp,
    "bundle_id": metadata["source_bundle"]["bundle_id"],
    "bundle_digest": bundle_digest,
    "verifier_commit": verifier_commit,
}
record_id = blake3(json.dumps(record_seed, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest()[:16]

pointer_path = publish_root / f"root-trust-pointer.{record_id}.json"
pointer = {
    "schema": "exo.root_trust_bundle_pointer.v1",
    "artifact_id": metadata["artifact_id"],
    "issued_at": install_timestamp,
    "verification_status": "verified",
    "verification_command": verification_command,
    "verifier_commit": verifier_commit,
    "trusted_verifier_commit": verifier_commit,
    "trusted_verifier_commit_source": metadata["trusted_verifier_commit_source"],
    "source_bundle_repo_commit": metadata["source_bundle_repo_commit"],
    "verifier_version": cargo_version,
    "artifact_uri": bundle_uri,
    "bundle_checksum": {
        "algorithm": "BLAKE3",
        "value": bundle_digest,
    },
    "bundle_id": metadata["source_bundle"]["bundle_id"],
    "bundle_id_hex": metadata["source_bundle"]["bundle_id_hex"],
    "bundle_format": metadata["bundle_format"],
    "source_path": metadata["source_path"],
    "pointer_file": pointer_path.name,
    "record_id": record_id,
    "frozen": {
        "read_only": True,
    },
}
pointer["pointer_checksum"] = {
    "algorithm": "BLAKE3",
    "value": blake3(json.dumps(pointer, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest(),
}
pointer_path.write_text(json.dumps(pointer, sort_keys=True, indent=2) + "\n", encoding="utf-8")

record = {
    "record_id": record_id,
    "recorded_at": install_timestamp,
    "artifact_id": metadata["artifact_id"],
    "artifact_uri": bundle_uri,
    "source_path": metadata["source_path"],
    "source_checksum": metadata["source_checksum"],
    "source_sha256": metadata["source_sha256"],
    "bundle_id": metadata["source_bundle"]["bundle_id"],
    "bundle_id_hex": metadata["source_bundle"]["bundle_id_hex"],
    "bundle_format": metadata["bundle_format"],
    "ceremony": {
        "ceremony_id": metadata["source_bundle"]["ceremony_id"],
        "network_id": metadata["source_bundle"]["network_id"],
        "repo_commit": metadata["source_bundle"]["repo_commit"],
        "constitution_hash": metadata["source_bundle"].get("constitution_hash"),
        "threshold": metadata["source_bundle"]["threshold"],
        "max_signers": metadata["source_bundle"]["max_signers"],
        "created_at": metadata["source_bundle"].get("created_at"),
    },
    "root_signature_signers": metadata["root_signature_signers"],
    "bundle_checksum": {
        "algorithm": "BLAKE3",
        "value": bundle_digest,
    },
    "canonical_bundle_checksum": metadata["canonical_bundle_checksum"],
    "verification": {
        "command": verification_command,
        "command_version": verifier_commit,
        "command_toolchain": cargo_version,
        "result": "verified",
        "pointer_file": pointer_path.name,
        "source_bundle_repo_commit": metadata["source_bundle_repo_commit"],
        "trusted_verifier_commit": verifier_commit,
        "trusted_verifier_commit_source": metadata["trusted_verifier_commit_source"],
    },
    "policy": {
        "source_type": "imported evidence",
        "fail_closed": True,
        "verifier_commit_authority": "operator trusted policy, never imported bundle contents",
        "allowed_exochain_trust_claims": [
            "only after downstream equivalent verification by a trusted verifier commit"
        ],
        "rollback_path": "delete artifact/pointer and re-run install with replacement manifest",
    },
}

if manifest_path.exists():
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise SystemExit("manifest must be a JSON object")
    if manifest.get("artifact_id") != metadata["artifact_id"]:
        raise SystemExit("manifest artifact_id mismatch")
else:
    manifest = {
        "schema": "exo.root_trust_install_manifest.v1",
        "artifact_id": metadata["artifact_id"],
        "records": [],
        "policy": {
            "append_only": True,
            "trust_source": "imported evidence",
            "allowed_exochain_claims": "none until downstream runtime verifier re-checks",
            "required_runtime_check": "exo-node verify-bundle with trusted verifier commit",
            "verifier_commit_authority": "operator trusted policy, never imported bundle contents",
        },
    }

manifest.pop("manifest_checksum", None)
manifest.setdefault("records", []).append(record)
manifest["latest_record_id"] = record_id
manifest["latest_recorded_at"] = install_timestamp
manifest["records_count"] = len(manifest["records"])
manifest["updated_at"] = install_timestamp
manifest["manifest_checksum"] = {
    "algorithm": "BLAKE3",
    "value": blake3(json.dumps(manifest, sort_keys=True, separators=(",", ":")).encode("utf-8")).hexdigest(),
}
manifest_path.write_text(json.dumps(manifest, sort_keys=True, indent=2) + "\n", encoding="utf-8")

print(record_id)
PY_RECORD
)"

if [ -z "$record_id" ]; then
  fail "Failed to persist installation metadata"
fi

pointer_path="$publish_root/root-trust-pointer.${record_id}.json"
if [ ! -f "$pointer_path" ]; then
  fail "Pointer file missing after install: $pointer_path"
fi

for path in "$publish_bundle" "$pointer_path" "$manifest_path"; do
  chmod 444 "$path"
  [ -f "$path" ] || fail "Publish artifact missing after install: $path"
done

bundle_id_hex="$(python3 - "$metadata_path" <<'PY_BUNDLE_ID'
import json
import sys
from pathlib import Path

metadata = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
print(metadata["source_bundle"]["bundle_id_hex"])
PY_BUNDLE_ID
)"
bundle_checksum="$(file_blake3 "$publish_bundle")"

cat <<EOF_SUMMARY
root trust bundle install complete
artifact-id: $artifact_id
record-id: $record_id
bundle-id: $bundle_id_hex
bundle-blake3: $bundle_checksum
verifier-commit: $verifier_commit
source-bundle-repo-commit: $source_bundle_repo_commit
published-at: $install_timestamp
artifact-uri: file://$publish_bundle
publish-root: $publish_root
manifest-path: $manifest_path
pointer-path: $pointer_path
EOF_SUMMARY
