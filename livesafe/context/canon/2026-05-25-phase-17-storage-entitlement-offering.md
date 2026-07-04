# Phase 17 Storage Entitlement Offering

## Source Basis

- Bob Stewart current-thread direction on 2026-05-25: storage on IPFS or other
  providers should be part of the initial product offering, and LiveSafe should
  charge for storage levels.
- Local implementation in `/Users/bobstewart/dev/livesafe/src/storage_entitlement.rs`.
- Local tests in
  `/Users/bobstewart/dev/livesafe/tests/storage_entitlement.rs`.

## Fact vs Inference

- Fact: Initial LiveSafe product packaging must include storage levels.
- Fact: IPFS or another content-addressed provider option belongs in the
  initial storage offering.
- Fact: Paid storage levels must be chargeable through the entitlement and
  billing model.
- Fact: Storage must preserve the existing raw-sensitive-data boundary.
- Inference: Content-addressed providers should receive encrypted blobs only,
  with opaque metadata and safe EXOCHAIN anchors.
- Inference: Tier-0 emergency reads should not depend on billing or quota state,
  while authorization remains mandatory.

## Artifact Inventory

| Artifact | Type | Location | Relevant concepts | Why it matters | Confidence | Action |
| --- | --- | --- | --- | --- | --- | --- |
| Bob storage-level direction | current-thread user direction | Codex thread, 2026-05-25 | IPFS, storage levels, paid offering | Establishes storage as initial product scope | high | preserve |
| Storage entitlement source | Rust source | `/Users/bobstewart/dev/livesafe/src/storage_entitlement.rs` | tiers, providers, quota, billing, anchors | Makes the storage requirement executable | high | use |
| Storage entitlement tests | Cargo tests | `/Users/bobstewart/dev/livesafe/tests/storage_entitlement.rs` | encrypted writes, IPFS option, Stripe gates, Tier-0 reads | Verifies accepted and denied storage behavior | high | run |
| Test plan | verification doc | `/Users/bobstewart/dev/livesafe/docs/TEST_PLAN.md` | storage acceptance gates | Extends onboarding and entitlement gates | high | use |

## Open Conflicts

- Exact storage prices, Stripe product ids, quota sizes, and provider contracts
  require commercial approval before production billing.
- The production storage provider mix is not final.
- Provider-specific compliance evidence and data-region controls need vendor
  evidence before PHI, genetic, or identity documents are processed.
