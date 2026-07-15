# EXOCHAIN Demo Adjacent Surface Intake

The demo workspace is proprietary adjacent software. It is not the canonical
EXOCHAIN Rust trust fabric. The sole Apache-2.0 exception in this subtree is
`demo/packages/exochain-wasm`, a wrapper around artifacts generated from the
core `crates/exochain-wasm` primitive.

## Accountability

- Owner: Exochain Foundation
- Accountable maintainer: repository maintainer on duty for demo changes
- Deployment status: `prototype`

## Trust boundary

- Constitutional trust claims allowed: none. A demo may describe an API call,
  but it may not claim constitutional enforcement without a tested call to the
  canonical Rust API or generated WASM adapter and tested fail-closed behavior.
- Core state access: demo services can submit authenticated, tenant-scoped DAG
  DB requests through the configured gateway adapter. They cannot mint or
  simulate consent, authority, provenance, governance, settlement, or legal
  outcomes outside core enforcement.
- Exact boundary: `demo/packages/shared/src/dagdb-adapter.js` transports demo
  requests to the configured gateway; `demo/packages/exochain-wasm` transports
  calls to the generated core WASM primitive. Every app, service, web page,
  compose file, script, and shared demo helper remains adjacent.

## Validation and operations

- Surface test commands: `npm --prefix demo ci`, `npm --prefix demo test`,
  `npm --prefix demo audit --audit-level=moderate`, and each app's documented
  build and surface-policy check.
- CI gate: `bash tools/test_demo_adjacent_boundaries.sh` in Gate 9, plus the
  demo tests and the core WASM bridge gate for the separate adapter boundary.
- Runtime configuration source: environment variables supplied outside Git.
- Secrets inventory: the eight `EXO_DEMO_DAGDB_*` gateway, authority, tenant,
  DID, and write-signature variables; `CROSSCHECKED_API_TOKENS`;
  `LIVESAFE_API_TOKENS`; and `POSTGRES_PASSWORD` only when the quarantined
  legacy fixture profile is enabled. Demo secrets must never share core
  signing, bootstrap, tenant, or emergency-override credentials.
- Rollback/disablement: stop or remove the demo service/hosting route. Omit the
  `legacy-postgres-fixture` profile to disable the legacy database fixture.
  Unset required adapter or token configuration to make affected routes fail
  closed. Canonical core state remains outside the demo rollback boundary.

## Licensing

- All adjacent demo code is `UNLICENSED` proprietary software.
- CrossChecked and LiveSafe require written commercial terms, an active
  EXOCHAIN bailment licensure record, and EXOCHAIN usage accounting.
- The Apache-2.0 core WASM wrapper exception does not license an adjacent app.
