# AVC Issue 713 Production Closure Proof

Status: Closed after PR #722 merge and deployment.

This packet records the production evidence and code-level hardening for GitHub
issue `#713`: intermittent AVC receipt emission fail-closing with RFC 3161 TSA
unavailability.

## Classification

- `crates/exo-node/src/avc.rs`: EXOCHAIN core runtime adapter.
- Railway HTTP logs, Postgres read-only queries, and GitHub issue comments:
  imported evidence.
- This file: constitutional governance/proof documentation.

## Production Runtime Evidence

Observed on 2026-06-29 against `https://exochain.io`:

- Railway production service `exochain` was deployed from
  `38d30e1a6c8a68581f6ae166f82642b9f4f88473` during the production smoke
  evidence window; PR `#722` later merged and deployed as
  `61f8b021fe29fb4fb945adedee362feab42aca66`.
- `GET /ready` returned `status: ok`, `dagdb_runtime_status: dagdb_active`,
  and `dagdb_runtime_reason: db_probe_ok`.
- Authenticated `GET /api/v1/avc/protocol` returned protocol version `1`,
  schema version `1`, and WASM package `@exochain/exochain-wasm`.
- Railway HTTP logs show 27/27 authenticated production `POST /api/v1/avc/receipts/emit` calls returned `200` after `#718`.
- Production AVC registry readback found 64 receipts for actor
  `did:exo:44sVCyeMCcef7PzAeM8jY7qpRUpmaQxabaC4etXeE9zr`.
- All 64 read back through `GET /api/v1/avc/receipts?actor=...` with
  `timestamp_provenance: ExternalTimestampAuthority` and RFC 3161 proof.
- The 27 post-`#718` trust-anchor receipts included 18 `signer_spki` anchors
  and 9 `issuing_ca_spki` anchors.
- `dagdb.dagdb_node_committed` and `dagdb.dagdb_node_trust_receipts` each
  contained 64 EXOCHAIN finality rows for `avc.receipt.exochain_finality`.

## Readback Samples

Signer-SPKI anchored sample:

- AVC receipt hash:
  `0c01f1dd4d2d78f81753caf86cd44ec0779e3c50b2cd79514a9188f49a234a7e`
- `GET /api/v1/avc/receipts/<hash>` returned `decision: Allow`,
  `timestamp_provenance: ExternalTimestampAuthority`, proof kind `Rfc3161`,
  authority DID `did:exo:microsoft-public-rsa-tsa`, action
  `archon.workflow.success`, actor DID
  `did:exo:44sVCyeMCcef7PzAeM8jY7qpRUpmaQxabaC4etXeE9zr`,
  and `tsa_trust_anchor_kind: signer_spki`.
- The receipt contained RFC 3161 token bytes and trust-anchor SPKI evidence.

Issuing-CA-SPKI anchored sample:

- AVC receipt hash:
  `29058510dbbe27d194cdb7f3784dcb3f5b90886bbe5632db0aae01b1a7878cbc`
- `GET /api/v1/avc/receipts/<hash>` returned `decision: Allow`,
  `timestamp_provenance: ExternalTimestampAuthority`, proof kind `Rfc3161`,
  authority DID `did:exo:microsoft-public-rsa-tsa`, action
  `archon.workflow.success`, actor DID
  `did:exo:44sVCyeMCcef7PzAeM8jY7qpRUpmaQxabaC4etXeE9zr`,
  and `tsa_trust_anchor_kind: issuing_ca_spki`.
- The receipt contained RFC 3161 token bytes and trust-anchor SPKI evidence.

Latest observed EXOCHAIN finality row:

- finality hash:
  `b20e1818d9963c52a37843cab2a888a97a147eead8a909521a6005efddc94439`
- finality height: `64`
- finality receipt hash:
  `14a00075ef03f7826f1c62b177938455e36fa4a9ef3d5f0cfb4181e78fedb008`

## Code Closure

The residual valid scope of `#713` after PRs `#714` and `#718` was genuine
transient RFC 3161 TSA fetch failure. PR `#722` adds bounded retry/backoff only
around the RFC 3161 HTTP fetch path:

- retryable: request transport failures, HTTP `5xx`, and HTTP `429`;
- not retryable: malformed DER, nonce/imprint/policy mismatch, trust-anchor
  mismatch, invalid proof, and all other verification failures;
- attempts: one initial attempt plus two bounded retries;
- production delays: 250 ms then 1000 ms;
- test delays: zero-duration, preserving fast deterministic tests.

## Verification

Commands recorded for PR `#722`:

```bash
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/bobstewart/dev/exochain/target cargo test -p exo-node rfc3161_timestamp_fetch_retries_transient_status_before_success -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/bobstewart/dev/exochain/target cargo test -p exo-node receipt_emit_does_not_retry_rfc3161_verification_failures -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/bobstewart/dev/exochain/target cargo test -p exo-node rfc3161 -- --nocapture
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/bobstewart/dev/exochain/target cargo test -p exo-node external_timestamp_error_surfaces_operator_class_in_public_message -- --nocapture
cargo fmt --all -- --check
git diff --check
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/bobstewart/dev/exochain/target cargo clippy -p exo-node --all-targets -- -D warnings
```

Results:

- new retry test: passed;
- new verification-failure no-retry test: passed;
- RFC 3161 test slice: 24 passed, 1 live Microsoft preflight ignored by design;
- operator-class diagnostic test: passed;
- format check: passed;
- diff whitespace check: passed;
- focused `exo-node` clippy: passed;
- `main` EXOCHAIN Constitutional CI for
  `61f8b021fe29fb4fb945adedee362feab42aca66`: passed.

## Disposition

`#713` is closed as `COMPLETED`. The production blocker is no longer open:
post-`#718` production emitted and read back authenticated RFC 3161 AVC receipts
with trust-anchor evidence, and PR `#722` completed the remaining bounded
retry/backoff hardening requested by the issue title.
