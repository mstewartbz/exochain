# AVC Root Trust Bundle Installation (Imported Evidence)

## Purpose

This runbook installs the imported ceremony artifact at `/Users/bobstewart/exo-ceremony/bundle.json` into
`artifacts/trust/avc-exo-ceremony-2026` using `tools/root-trust-install.sh`.

The installer performs:

1. preservation of the bundle exactly as `assemble-bundle` emitted it,
2. strict EXOCHAIN verification via `exo-node` at the bundle's declared `config.repo_commit`,
3. immutable publish of the verified emitted artifact,
4. manifest/pointer write for audit and fail-closed consumption.

## Prerequisites

- `cargo` available on PATH.
- `python3` and Python module `blake3`.
- Source artifact at `/Users/bobstewart/exo-ceremony/bundle.json`.

## Installation command

```bash
tools/root-trust-install.sh
```

Equivalent with explicit arguments:

```bash
tools/root-trust-install.sh \
  --source /Users/bobstewart/exo-ceremony/bundle.json \
  --artifact-id avc-exo-ceremony-2026 \
  --publish-root artifacts/trust/avc-exo-ceremony-2026
```

## Verification summary

On success, verify:

- `install-manifest.json` exists and is writable only after install.
- verified emitted bundle appears at `root-trust-bundle.canonical.json`.
- bundle pointer appears at `root-trust-pointer.<record-id>.json`.
- manifest record and pointer report `verification_status = "verified"` and the ceremony `repo_commit` used as the verifier commit.

## Fail-closed consumer rule

Consumers must ignore manifest entries unless:

- pointer/manifest are parseable JSON,
- pointer checksum matches recomputed value,
- pointer `verification_status` is `verified`,
- published bundle exists at `artifact_uri`,
- published bundle BLAKE3 checksum matches pointer.

If any condition fails, the bundle must not be treated as trust-anchored.

## Test matrix

The required test cases are implemented in:

```bash
tools/test_root_trust_install_plan.sh
```

Scenarios covered:

1. Happy-path install validation.
2. Missing field failure for `transcript_hash`.
3. Signature tamper failure.
4. Identity/certificate tamper failure (`signer_ids`, `config.threshold`, or `config.ceremony_id`).
5. Deployment fail-closed check when published bundle is missing.
