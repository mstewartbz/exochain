# LiveSafe VitalLock Vault Model

## Source Basis

- `docs/context/LIVESAFE_PRODUCT_ARCHITECTURE.md`
- `docs/context/LIVESAFE_STORAGE_ENTITLEMENTS_AND_VAULT_PROVIDERS.md`
- `docs/context/LIVESAFE_MEDICAL_JACKET_AND_CUSTODY_MODEL.md`
- `docs/context/LIVESAFE_ESCALATIONS_FOR_BOB.md`
- `docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md`
- `context/canon/2026-05-24-round-1-collective-context.md`
- `src/vitallock_vault.rs`
- `tests/vitallock_vault.rs`

## Ground Truth

Current repo truth identifies VitalLock as the protected vault lineage inside
the broader LiveSafe product architecture. The repo already had storage
entitlement rules and medical-jacket custody rules, but it did not yet define a
dedicated adjacent-surface contract for VitalLock vault interactions.

This new contract remains adjacent-surface only. It does not activate EXOCHAIN
runtime authority, does not authorize public trust claims, and does not permit
raw medical, genetic, identity, contact, location, or emergency payloads in
fixtures or interaction metadata.

## Current Contract Coverage

`src/vitallock_vault.rs` now defines fail-closed vocabulary for:

- owner reads and writes,
- Tier-0 responder reads,
- P.A.C.E. delegated reads,
- medical-jacket, consent-receipt, P.A.C.E. contact, and emergency-instruction
  pointer classes,
- metadata-only, emergency-subset, and blocked full-export disclosure scopes,
- verified-permit versus inactive runtime posture.

The contract requires synthetic vault, record, policy, and session references
for every interaction.

## Storage And Custody Boundary

VitalLock vault interactions now depend on current contract evidence rather
than free-form runtime assumptions:

- every interaction requires a passing storage entitlement contract,
- medical-jacket and consent-bound interactions require a passing custody or
  consent contract,
- interaction payloads must stay metadata-only,
- direct contact values and location traces are denied,
- verified protection claims are blocked unless the runtime state is
  `VerifiedPermit`.

This keeps the vault surface aligned with the existing storage and
medical-jacket doctrine instead of inventing an independent trust path.

## Responder And Delegated Access Boundary

Current repo truth still treats live first-responder disclosure scope as
Bob-only. The implemented vault contract therefore stays fail-closed:

- Tier-0 responder access is limited to `EmergencySubset`,
- P.A.C.E. delegated access is limited to `MetadataOnly`,
- delegated and responder-facing access require a disablement reference,
- delegated and responder-facing access require active authorization,
- delegated and responder-facing access remain inactive unless a verified
  adapter path returns permit,
- full vault export remains blocked until a verified export policy exists.

## Test Evidence

- `cargo test --test vitallock_vault`
- `npm run quality`

## Boundaries

- Classification: adjacent surface domain contract and control documentation.
- Trust posture: inactive for public VitalLock or EXOCHAIN-backed protection
  claims until a verified permit path exists.
- Data posture: synthetic references only; no raw medical, genetic, identity,
  contact, location, or emergency payloads.
- Disablement path: keep `src/vitallock_vault.rs` unwired from runtime routes
  or remove the `vitallock_vault` export from `src/lib.rs`.
