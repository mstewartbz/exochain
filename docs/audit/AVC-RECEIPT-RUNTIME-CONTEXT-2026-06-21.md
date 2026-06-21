# AVC Receipt Runtime Context - 2026-06-21

## Purpose

This note records the database, timestamp authority, and remediation answers
needed by future EXOCHAIN automation runs. Treat it as operator context and
deployment evidence; the authoritative implementation is the Rust code and the
test suite.

## Path Classification

- `docs/audit/AVC-RECEIPT-RUNTIME-CONTEXT-2026-06-21.md`: core runtime adapter
  operations context and deployment evidence.
- `crates/exo-avc/src/*`: EXOCHAIN core AVC receipt and validation primitives.
- `crates/exo-node/src/avc.rs`: core runtime adapter for the AVC HTTP route.

## Recorded Time

- Record updated: `2026-06-21T04:22:01Z`.
- Railway config deployment observed: `2026-06-21T03:40:12.028Z`.

## Railway Target

- Workspace: `ARMORCLOUD`.
- Railway project: `exochain`.
- Railway project id: `ca52ac39-820a-488b-8f29-df17d76a9270`.
- Environment: `production`.
- Service: `exochain`.
- Service id: `e6538b78-5c05-4b37-b308-57a1249ad243`.
- Latest deployment id at verification time: `7038665e-65ba-467a-b444-3d558c60877a`.
- Latest deployment status at verification time: `SUCCESS`.

## Database Answer

Railway production read-back showed:

```text
DATABASE_URL set
EXO_AVC_REQUIRE_POSTGRES_DURABILITY set
```

The raw `DATABASE_URL` secret was not printed and must not be committed.

`DATABASE_URL` is the production durability floor for AVC runtime records. With
it configured and `EXO_AVC_REQUIRE_POSTGRES_DURABILITY=true`, production startup
must fail closed if Postgres durability is unavailable instead of silently using
the local file fallback.

`DATABASE_URL` is not a trusted timestamp authority. It does not provide
independent external time, RFC 3161 timestamp tokens, eIDAS qualified
timestamping, blockchain anchoring, or third-party notarization.

## Timestamp Authority Answer

Production read-back did not show these variables as set:

```text
EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_URL
EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_DID
EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_PUBLIC_KEY_HEX
```

The runtime receipt route is therefore expected to fail closed on receipt
emission until all three values are configured together. This is intentional:
EXOCHAIN code can verify the authority signature and bind it into a receipt, but
it must not pretend that EXOCHAIN's own database clock or local HLC is an
independent timestamp authority.

The production receipt boundary is:

1. Canonicalize and hash the AVC action descriptor.
2. Canonicalize and hash the receipt evidence subject: credential id, action id,
   action commitment hash, action descriptor hash, and prior receipt hash.
3. Request a signed external timestamp proof for that evidence-subject hash.
4. Verify the returned authority DID, subject hash, public-key signature, and
   timestamp.
5. Validate the AVC at the externally issued timestamp.
6. Mint the EXOCHAIN validator-signed receipt with the embedded action
   descriptor, descriptor hash, previous-receipt link, and external timestamp
   proof.
7. Store it durably through the configured registry backend.

## Court-Grade Labeling

- `operational_exochain_receipt`: EXOCHAIN-signed receipt with runtime
  provenance but no independent external timestamp.
- `ordered_exochain_receipt`: EXOCHAIN-signed receipt with receipt-chain or
  DAG/BCTS ordering proof.
- `court_grade_external_time`: EXOCHAIN-signed, ordered receipt with a verified
  independent timestamp authority proof and external anchoring evidence.

Do not describe Postgres `clock_timestamp()`, local HLC, caller-supplied
request time, or app-reported wall-clock time as court-grade external time.

## Issue Mapping

- GitHub issue #700: database durability is configured in Railway production;
  code still fails closed if required Postgres durability is unavailable.
- GitHub issue #699: receipts now embed a structured AVC action descriptor and
  descriptor hash, not only opaque metadata plus a commitment hash.
- GitHub issue #698: receipt emission now requires a verified external
  timestamp-authority proof; missing authority config returns a fail-closed
  service error.
- GitHub issue #697: receipts retain the prior-receipt link and bind that link
  into the external evidence-subject hash, giving the local receipt chain an
  externally signed timestamp anchor per emitted receipt.
- GitHub issue #694: DAG DB governed memory adapter status remains separate
  from AVC receipt remediation; current repo verification shows the production
  DAG DB route/finality/consent checks passing under the `production-db`
  feature.

## Verification Commands

```bash
cargo test -p exo-avc
cargo test -p exo-node avc::tests::
RUSTFLAGS='-D warnings' cargo test -p exo-dag-db-domain --test prd17_default_retrieval_contract --test prd17_lifecycle_contract
RUSTFLAGS='-D warnings' cargo test -p exo-gateway dagdb --features production-db
RUSTFLAGS='-D warnings' cargo check -p exo-gateway --features production-db
```

Safe Railway read-back, without printing secret values:

```bash
railway variable list \
  --project ca52ac39-820a-488b-8f29-df17d76a9270 \
  --environment production \
  --service exochain \
  --json \
  | jq -r 'to_entries[] | select(.key == "DATABASE_URL" or .key == "EXO_AVC_REQUIRE_POSTGRES_DURABILITY" or .key == "EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_URL" or .key == "EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_DID" or .key == "EXO_AVC_EXTERNAL_TIMESTAMP_AUTHORITY_PUBLIC_KEY_HEX") | [.key, (if ((.value|tostring|length) > 0) then "set" else "empty" end)] | @tsv'
```
