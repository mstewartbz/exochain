# LiveSafe Storage Entitlements And Vault Providers

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/LIVESAFE_AUTOMATION_READINESS.md`
- `docs/context/LIVESAFE_CONTEXT_SEED.md`
- `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`
- `context/canon/2026-05-25-phase-17-storage-entitlement-offering.md`
- `src/storage_entitlement.rs`
- `tests/storage_entitlement.rs`

## Ground Truth

LiveSafe treats storage as part of the initial product offering, not as a later
add-on. The current contract evidence in `src/storage_entitlement.rs` and
`tests/storage_entitlement.rs` defines included and paid storage levels,
encrypted provider-write requirements, safe EXOCHAIN anchor fields, quota
enforcement, and Tier-0 emergency reads.

This is adjacent-surface contract work only. It does not activate EXOCHAIN
runtime authority, does not authorize public trust claims, and does not permit
raw sensitive medical, genetic, identity, contact, emergency, eligibility, or
payment records in repo fixtures, provider metadata, or EXOCHAIN anchors.

## Initial Storage Offering

The current storage contract records these initial LiveSafe storage levels:

| Storage level | Contract code | Billing mode | Initial offering | Current quota |
| --- | --- | --- | --- | --- |
| Basic Included Vault Storage | `BasicIncluded` | `Included` | yes | `512 MiB` |
| Personal Paid Vault Storage | `PersonalPaid` | `StripeRecurring` | yes | `10,240 MiB` |
| Family Paid Vault Storage | `FamilyPaid` | `StripeRecurring` | yes | `51,200 MiB` |
| Team Paid Vault Storage | `TeamPaid` | `StripeRecurring` | yes | `204,800 MiB` |
| Enterprise Custom Vault Storage | `EnterpriseCustom` | `CustomContract` | no | `1,048,576 MiB` |

The initial offering must include the four initial codes above and must include
at least one content-addressed option. The current contract satisfies that by
including `ProviderKind::IpfsContentAddressed` and
`ProviderKind::FilecoinContentAddressed` across the initial offering.

Paid storage levels require explicit commercial binding. In current repo truth,
that means Stripe-backed tiers require Stripe catalog binding, while enterprise
storage remains custom-contract only. Exact Stripe product ids and price ids
remain Bob-controlled production configuration, not repo fixtures.

## Provider Boundaries

The current provider set in `src/storage_entitlement.rs` is:

- `ProviderKind::IpfsContentAddressed`
- `ProviderKind::FilecoinContentAddressed`
- `ProviderKind::S3CompatibleObjectStore`
- `ProviderKind::ManagedVaultStore`

All provider classes share these hard boundaries:

- Vault providers may receive only encrypted blobs.
- Raw sensitive data must not be written to IPFS, content-addressed storage,
  object storage, logs, or fixtures.
- Provider metadata must not include human-readable sensitive labels.
- Paid storage writes require current billing, active trial, gift, or
  frontline entitlement.
- Storage writes must fail closed when quota is exceeded.

These rules are source-backed by `tests/storage_entitlement.rs`, including the
denial path for unencrypted IPFS-style writes and the allowed path for
encrypted content-addressed writes with safe metadata only.

## Safe EXOCHAIN Anchor Fields

Current contract truth allows only these storage-anchor fields:

- `cid`
- `commitment`
- `custody-receipt`
- `policy-reference`
- `retention-policy-reference`
- `encryption-key-commitment`

This keeps LiveSafe storage anchors at the commitment/reference layer only.
Unsafe fields, including any raw medical or human-readable record content, are
denied. Storage contracts therefore preserve the adjacent-surface doctrine that
raw sensitive records remain off-chain even when storage is billable and
content-addressed.

## Tier-0 Emergency Read Boundary

Tier-0 emergency reads are intentionally narrower than general vault access.
The current storage-access contract allows Tier-0 emergency reads to bypass
billing and quota checks, but not authorization.

Tier-0 emergency reads still require an authorized access state. If
authorization is denied, expired, or revoked, the read fails closed. This keeps
emergency-read posture aligned with the repo doctrine: emergency access may be
fast, but it is never anonymous, public, or trust-claiming.

## Commercial And Deployment Constraints

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- Source basis for storage packaging comes from
  `context/canon/2026-05-25-phase-17-storage-entitlement-offering.md` and
  `docs/context/LIVESAFE_COMMERCIAL_ENTITLEMENTS_AND_MARKETPLACE.md`.
- Production provider mix, provider-region/compliance evidence, Stripe product
  ids, Stripe price ids, quota sizes, and storage pricing remain unresolved
  commercial or vendor decisions.
- Disablement path: keep storage behavior bound to the current contract layer,
  avoid wiring provider writes to public trust claims, and deny any provider or
  anchor shape that falls outside `src/storage_entitlement.rs`.
