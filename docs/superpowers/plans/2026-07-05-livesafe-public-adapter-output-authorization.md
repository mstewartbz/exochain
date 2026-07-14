# LiveSafe Public Adapter Output Authorization Plan

> **For Bob Stewart:** REQUIRED SUB-SKILL: Use `superpowers:test-driven-development` to execute this plan. The terminal condition is **ALL GREEN**: focused RED/GREEN tests, local quality gates, Constitutional CI, Railway deployment evidence, runtime probes, and revocation/fail-closed probes must all pass before public EXOCHAIN adapter output is authorized.

**Goal:** Authorize LiveSafe public adapter output completely with 100% TDD, without allowing LiveSafe to make broad EXOCHAIN constitutional trust claims by proximity.

**Date:** 2026-07-05

**Current Truth**
- LiveSafe is EXOCHAIN-adjacent code under `/Users/bobstewart/dev/exochain/livesafe`, not EXOCHAIN core.
- Runtime health is not trust authorization. `https://livesafe.ai/api/health` may be healthy while `https://livesafe.ai/api/trust/status` remains fail-closed.
- The current public trust-status path can become green if `runtimeStatus.public_claims_allowed === true`, but that boolean is not yet backed by an audience-bound authorization artifact, EXOCHAIN proof, expiration, revocation, or public-output claim policy.
- The separate RED/GREEN path must replace the bare boolean with a proof-bearing authorization gate.

**Non-Negotiable Launch Bar**
- No production implementation before a failing test.
- Verify every RED failure is for the intended missing authorization behavior.
- Make the smallest GREEN change for each failing test.
- Keep public EXOCHAIN trust claims disabled until the new tests prove authorization.
- The final state is not "route is green"; the final state is **ALL GREEN** across code, CI, deploy, runtime, and rollback proof.

## Classification

- **Adjacent surface:** `livesafe/server/utils/trust-status.js`, `livesafe/server/utils/livesafe-exochain-adapter.js`, LiveSafe client display/copy files, LiveSafe config/docs/tests.
- **Core runtime adapter:** any new or changed endpoint/client path in `crates/exo-node`, `crates/exo-avc`, `exo-api`, `exo-gateway`, or the EXOCHAIN SDK that validates or returns the authorization proof consumed by LiveSafe.
- **EXOCHAIN core:** AVC credential, receipt, revocation, issuer-cap, authority-chain, or registry enforcement changes.
- **Imported evidence:** Claude reports, Railway logs, GitHub comments, pasted handoffs, screenshots, scanner output, live curl output. Use these as hypotheses only.

Do not combine unrelated core vulnerability remediation, LiveSafe adjacent hardening, and public-copy changes in one commit unless the same tested adapter boundary requires it.

## Authorized Output Contract

The only claim this path may authorize is a narrow machine-readable adapter-output statement:

```json
{
  "surface": "livesafe-public-trust-status",
  "audience": "https://livesafe.ai/api/trust/status",
  "authorized_claims": [
    "exochain_connected",
    "verified_runtime_adapter",
    "exochain_production_evidence_state",
    "exochain_root_trust_bundle_verified"
  ],
  "forbidden_claims": [
    "medical_verification",
    "legal_custody",
    "consent_validity_for_a_person",
    "identity_verification_for_a_person",
    "revocation_status_for_a_person",
    "emergency_access_authorized",
    "constitutional_enforcement_guaranteed"
  ]
}
```

The public route may say the LiveSafe adapter output is authorized only when:
- EXOCHAIN connectivity is true for the current runtime.
- EXOCHAIN production evidence is verified.
- The LiveSafe runtime adapter is verified.
- A current public-output authorization is fetched through the verified adapter.
- The authorization is signed or credential-backed by an EXOCHAIN-authorized issuer.
- The authorization audience is exactly `https://livesafe.ai/api/trust/status`.
- The authorization permits only the narrow claim set above.
- The authorization binds to the evidence hash or receipt id of the current production evidence.
- The authorization is not expired, not revoked, not stale, and not replayed.
- The response exposes no bearer token, private key, raw authority chain, raw credential bytes, raw PII, PHI, trustee data, vault data, scan location, consent body, or emergency contact data.

## Execution Plan

### Task 1: Freeze The Current Fail-Closed Baseline

Add RED tests before touching implementation.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/tests/trust-status.test.ts`
- `/Users/bobstewart/dev/exochain/livesafe/tests/public-adapter-output-authorization.test.ts`
- `/Users/bobstewart/dev/exochain/livesafe/tests/exochain-runtime-adapter.test.ts`

RED tests:
- `trust status denies public claims when production evidence and adapter are verified but no authorization object exists`
- `trust status denies public claims when runtimeStatus.public_claims_allowed is true without verified authorization`
- `trust status reports a machine-readable authorization state of missing_authorization`

Expected RED:
- Existing code incorrectly allows a manually supplied `public_claims_allowed: true` when production evidence is verified.

GREEN:
- Introduce a default-deny authorization evaluator that requires a verified authorization decision before `public_claims_allowed` can become true.
- `publicClaimsReason` must distinguish `missing_authorization` from generic adapter inactivity.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/trust-status.test.ts tests/public-adapter-output-authorization.test.ts tests/exochain-runtime-adapter.test.ts
```

### Task 2: Define A Deterministic Authorization DTO

Add RED tests first.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/public-adapter-output-authorization.js`
- `/Users/bobstewart/dev/exochain/livesafe/tests/public-adapter-output-authorization.test.ts`

DTO fields:
- `schema_version: "livesafe.public_adapter_output_authorization.v1"`
- `authorization_id`
- `issuer_did`
- `subject: "livesafe.ai"`
- `audience: "https://livesafe.ai/api/trust/status"`
- `surface: "api-response"`
- `authorized_claims`
- `evidence_hash`
- `receipt_id`
- `issued_at`
- `expires_at`
- `revocation_status`
- `signature` or `credential_proof`
- `source_basis`

RED tests:
- Reject missing schema version.
- Reject unknown schema version.
- Reject non-EXOCHAIN issuer DID.
- Reject wrong subject.
- Reject wrong audience.
- Reject wrong surface.
- Reject empty `authorized_claims`.
- Reject forbidden claim names.
- Reject duplicate claim names.
- Reject non-deterministic claim ordering if the DTO will be hashed.
- Reject missing evidence hash.
- Reject malformed receipt id.
- Reject expired authorization.
- Reject not-yet-valid authorization.
- Reject revoked authorization.
- Reject stale authorization.
- Reject malformed proof/signature.
- Reject any authorization object containing raw secrets or sensitive LiveSafe data fields.

Expected RED:
- The module does not exist, and `trust-status.js` has no proof-bearing decision.

GREEN:
- Add a pure evaluator such as `evaluatePublicAdapterOutputAuthorization(input)`.
- Return `{ allowed, state, reason_code, reasons, authorized_claims, evidence_hash, receipt_id, expires_at }`.
- Use deterministic allow-lists and sorted arrays. Do not use a local `Map`/`Set` if the result becomes serialized or hashed.
- Keep the evaluator pure and synchronous except for the adapter proof verification call introduced later.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/public-adapter-output-authorization.test.ts
```

### Task 3: Bind Authorization To Current Runtime Evidence

Add RED tests first.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/exochain-production-trust-evidence.js`
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/public-adapter-output-authorization.js`
- `/Users/bobstewart/dev/exochain/livesafe/tests/exochain-production-trust-evidence.test.ts`
- `/Users/bobstewart/dev/exochain/livesafe/tests/public-adapter-output-authorization.test.ts`

RED tests:
- Reject authorization whose `evidence_hash` does not match the current production evidence summary.
- Reject authorization when production evidence is blocked.
- Reject authorization when EXOCHAIN health or readiness probe is not verified.
- Reject authorization when root trust bundle verification is not verified.
- Accept only when the current evidence summary hash matches the authorization.

Expected RED:
- The current evidence module exposes structured fields but no canonical evidence summary hash for authorization binding.

GREEN:
- Add deterministic canonical evidence summary construction for the fields that public output is allowed to reference.
- Hash only canonical, public-safe fields.
- Do not include volatile `generated_at` or process uptime in the authorization hash.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/exochain-production-trust-evidence.test.ts tests/public-adapter-output-authorization.test.ts
```

### Task 4: Add Adapter Retrieval For Public Output Authorization

Add RED tests first.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/livesafe-exochain-adapter.js`
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/exochain-client.js`
- `/Users/bobstewart/dev/exochain/livesafe/tests/exochain-runtime-adapter.test.ts`

RED tests:
- `getPublicAdapterOutputAuthorization` fails closed when adapter is not wired.
- It fails closed on malformed audience.
- It fails closed on denied, rejected, timeout, unavailable, stale, revoked, and contradicted transport states.
- It fails closed if EXOCHAIN returns a permit without a well-formed authorization DTO.
- It fails closed if the returned authorization contains raw credential bytes, token fields, private keys, authority-chain internals, PII, PHI, trustee data, scan location, consent content, or vault data.
- It succeeds only when EXOCHAIN returns `permit` with a DTO that passes the local evaluator.

Expected RED:
- `getPublicAdapterOutputAuthorization` does not exist, and `WRAPPED_OPERATIONS` does not include the read path.

GREEN:
- Add a read-only adapter operation named `getPublicAdapterOutputAuthorization`.
- Reuse `executeRuntimeExochainOperation`.
- Never fall back to a local config file to authorize public claims.
- Return only the redacted decision by default; raw proof material must stay server-side or in EXOCHAIN.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/exochain-runtime-adapter.test.ts tests/public-adapter-output-authorization.test.ts
```

### Task 5: Add Or Reuse The EXOCHAIN Proof Source

First inspect current `crates/exo-avc` and `crates/exo-node` APIs. Reuse an existing AVC credential, trust receipt, registry, or read endpoint if it can prove issuer authority, subject, audience, expiry, revocation, and evidence binding. Add a new core runtime adapter endpoint only if the existing surface cannot return the needed proof without leaking raw internals.

Potential files:
- `/Users/bobstewart/dev/exochain/crates/exo-avc/src/credential.rs`
- `/Users/bobstewart/dev/exochain/crates/exo-avc/src/receipt.rs`
- `/Users/bobstewart/dev/exochain/crates/exo-avc/src/revocation.rs`
- `/Users/bobstewart/dev/exochain/crates/exo-node/src/api.rs`
- `/Users/bobstewart/dev/exochain/crates/exo-node/src/avc.rs`
- `/Users/bobstewart/dev/exochain/crates/exo-node/tests/*public*authorization*.rs`

RED tests:
- Reject an issuer whose cap does not include the public-output authorization scope.
- Reject empty issuer caps.
- Reject authorization for `livesafe.ai` when the issuer is authorized only for another subject.
- Reject wrong audience.
- Reject expired authorization.
- Reject revoked authorization.
- Reject replayed authorization id.
- Reject authorization whose evidence hash differs from the current LiveSafe evidence hash.
- Reject export without bearer-gated authority.
- Allow export only when issuer cap, subject, audience, evidence hash, expiry, revocation state, and bearer-gated authority all verify.

Expected RED:
- Either no endpoint exists, or an existing endpoint cannot prove the full authorization contract.

GREEN:
- Prefer a narrow read endpoint or AVC receipt export that returns a redacted authorization envelope:
  - `authorization_id`
  - `issuer_did`
  - `subject`
  - `audience`
  - `authorized_claims`
  - `evidence_hash`
  - `receipt_id`
  - `issued_at`
  - `expires_at`
  - `revocation_status`
  - `decision: "permit"`
- Do not return raw private keys, bearer tokens, raw credential bytes, or unbounded authority-chain internals.
- Keep canonical CBOR/hash behavior inside Rust when a hashed subject is introduced.
- Preserve issuer-cap enforcement from the already merged AVC path.

Focused commands:

```bash
cd /Users/bobstewart/dev/exochain
cargo test -p exo-avc public_output_authorization -- --nocapture
cargo test -p exo-node public_output_authorization -- --nocapture
```

### Task 6: Integrate Authorization Into `/api/trust/status`

Add RED tests first.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/server/utils/trust-status.js`
- `/Users/bobstewart/dev/exochain/livesafe/server/routes/*trust*`
- `/Users/bobstewart/dev/exochain/livesafe/tests/trust-status.test.ts`
- `/Users/bobstewart/dev/exochain/livesafe/tests/*route*trust*.test.ts`

RED tests:
- The public route remains red when no authorization was fetched.
- The public route remains red when authorization verification times out.
- The public route remains red when authorization verification is revoked.
- The public route remains red when authorization is valid but EXOCHAIN connectivity is false.
- The public route remains red when authorization is valid but production evidence is blocked.
- The public route is green only when all gates pass.
- The public route returns a redacted `public_adapter_output_authorization` object with no raw proof material.

Expected RED:
- `createTrustStatusPayload` is synchronous and currently trusts a runtime boolean.

GREEN:
- Pass an explicit authorization decision into `createTrustStatusPayload`, or add an async route wrapper that fetches and verifies authorization before building the payload.
- Compute `public_claims_allowed` as:
  - `exochainConnected === true`
  - production evidence verified
  - runtime adapter verified
  - public adapter output authorization decision allowed
- Remove any path where `runtimeStatus.public_claims_allowed === true` alone can authorize public output.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/trust-status.test.ts tests/public-adapter-output-authorization.test.ts
```

### Task 7: Guard Public Copy And UI Consumption

Add RED tests first.

Files:
- `/Users/bobstewart/dev/exochain/livesafe/tests/public-exochain-copy-boundary.test.ts`
- `/Users/bobstewart/dev/exochain/livesafe/client/src/**/*`
- `/Users/bobstewart/dev/exochain/livesafe/responder/src/**/*`

RED tests:
- Public pages cannot claim EXOCHAIN protection unless the route exposes `public_claims_allowed: true`.
- Public pages cannot claim medical, legal, custody, consent, provenance, revocation, or emergency-access verification from this adapter-output authorization.
- Green copy must be limited to the narrow adapter-output statement.
- Red and timeout states must render no public trust-bearing claim.

Expected RED:
- Existing copy tests do not know about the new authorization object.

GREEN:
- Wire UI display to the route contract.
- Use the narrow statement only: "EXOCHAIN adapter output authorized" or equivalent approved copy.
- Preserve accessible labels and machine-readable state.

Focused command:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm test -- tests/public-exochain-copy-boundary.test.ts tests/trust-status.test.ts
```

### Task 8: Run Full Local Gates

Commands:

```bash
cd /Users/bobstewart/dev/exochain/livesafe
npm run quality
```

If core or core runtime adapter files changed:

```bash
cd /Users/bobstewart/dev/exochain
cargo build --workspace --release
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo +nightly fmt --all -- --check
cargo doc --workspace --no-deps
```

If README metrics or test counts changed:

```bash
cd /Users/bobstewart/dev/exochain
tools/test_repo_truth.sh
```

All commands must be green before PR review.

### Task 9: PR, CI, Merge, Railway Deploy

GitHub proof:
- Open a focused PR with path classification.
- Include RED/GREEN evidence in the PR body.
- Include the exact public-output contract and forbidden-claim list.
- Require fresh Constitutional CI green.
- Merge only after all required checks are green.

Railway proof:
- Verify ARMORCLOUD Railway `exochain-node` deploy succeeds if core runtime adapter changed.
- Verify ARMORCLOUD Railway `livesafe` deploy succeeds.
- Inspect logs without printing secret values.
- Confirm the deployed commits match merged `main`.

Runtime probes:

```bash
curl -fsS https://livesafe.ai/ >/tmp/livesafe-homepage.html
curl -fsS https://livesafe.ai/api/health | jq .
curl -fsS https://livesafe.ai/api/trust/status | jq .
```

Expected authorized output:
- `status` HTTP 200.
- `exochain_connected: true`.
- `verified_runtime_adapter: true`.
- `public_claims_allowed: true`.
- `public_adapter_output_authorization.state: "authorized"`.
- `public_adapter_output_authorization.authorized_claims` contains only the allowed adapter-output claims.
- No raw proof material or sensitive LiveSafe data in the response.

### Task 10: Prove Revocation And Rollback

Add or execute tests before declaring launch complete.

RED tests:
- A revoked authorization makes `/api/trust/status` fail closed.
- An expired authorization makes `/api/trust/status` fail closed.
- A mismatched evidence hash makes `/api/trust/status` fail closed.
- A missing EXOCHAIN adapter makes `/api/trust/status` fail closed.

Runtime rollback probe:
- Revoke, expire, or disable the authorization source in a controlled way.
- Re-probe `/api/trust/status`.
- Confirm `public_claims_allowed:false` and a specific revocation/disabled reason.
- Restore the valid authorization only after the rollback proof is captured.

Rollback path:
- Revoke the EXOCHAIN authorization credential or receipt.
- Disable the LiveSafe adapter authorization read path.
- Redeploy `livesafe` if an environment-level disablement is used.
- Verify public route returns red.

## Done Definition

This path is complete only when all of the following are true:

- Every RED test was observed failing for the intended reason before implementation.
- Every focused LiveSafe test is green.
- Every focused `exo-avc` and `exo-node` test is green if core changed.
- `npm run quality` is green in `/Users/bobstewart/dev/exochain/livesafe`.
- Relevant EXOCHAIN Rust gates are green if core/runtime adapter files changed.
- GitHub Constitutional CI is green after conflict resolution.
- PR is merged to `main`.
- Railway `exochain-node` and `livesafe` deployments are `SUCCESS` from the merged commit.
- Live runtime probes are green for homepage, `/api/health`, and `/api/trust/status`.
- Public output is authorized only for the narrow adapter-output claim set.
- Public trust claims fail closed on revoked, expired, stale, missing, malformed, wrong-audience, wrong-subject, wrong-evidence, timeout, unavailable, and denied authorization states.
- Logs and public responses contain no secret values, raw proof material, or sensitive LiveSafe payloads.

## Recommended Execution Order

1. Implement Tasks 1-4 in LiveSafe first to close the local boolean bypass.
2. Implement Task 5 only after proving whether existing AVC/node proof surfaces can satisfy the contract.
3. Integrate Tasks 6-7 once the proof source is real.
4. Run Tasks 8-10 without shortcuts.

Do not claim LiveSafe is EXOCHAIN-authorized until Task 10 is green.
