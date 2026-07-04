# LiveSafe Genesis Development Trust

## Source Basis

- Bob Stewart current-thread direction on 2026-05-25: ExoForge may be trusted
  for all development efforts.
- Bob Stewart current-thread direction on 2026-05-25: the 7-of-13 FROST keygen
  ceremony is expected during the week of 2026-05-25.
- Bob Stewart current-thread direction on 2026-05-25: coding starts now, while
  external trust signaling remains blocked until internal proof exists.
- Local EXOCHAIN evidence: `/Users/bobstewart/dev/exochain/exochain/exoforge`
  classifies ExoForge as an adjacent implementation factory and proposal
  automation tool.

## Posture

ExoForge is trusted for its purpose: internal development planning,
implementation workflow, review routing, validation support, and bounded
development execution.

That internal development trust does not create public, customer-facing, legal,
medical, EXOCHAIN runtime, root, settlement, ratification, custody, consent,
authority, or provenance claims.

Outward trust state must be visually symbolic. Until a claim is verified, any
public or customer-facing output must display an AVC-style badge, lock-style
symbol, color/glow treatment, human-readable status, and machine-readable state
that makes the unverified status obvious at a glance.

## Genesis Rule

LiveSafe development may continue during genesis. Engineers and agents may use
ExoForge and source-backed repo evidence to build production-quality internal
contracts, deterministic fixtures, inactive trust-state views, fail-closed
adapters, product surfaces, and tests.

External trust signaling remains disabled until all required proof exists:

1. Source provenance for the development input.
2. Completed internal proof gate.
3. Completed 7-of-13 FROST keygen ceremony.
4. Verified runtime adapter for the specific claim.
5. Fail-closed tests for denial, timeout, and unavailable paths.
6. No raw sensitive data in logs, fixtures, receipts, or exported artifacts.

## FROST Ceremony Profile

The genesis ceremony profile is:

- Scheme: FROST
- Threshold: 7
- Participants: 13
- Current status: scheduled for the week of 2026-05-25
- External trust signal: disabled until completion and verification

## Development Trust Uses Allowed Now

- TDD implementation slices.
- Source-backed planning and implementation maps.
- ExoForge triage and planning outputs.
- Synthetic fixtures.
- Local contract modules.
- Inactive trust-state UI.
- Fail-closed adapter boundaries.
- Private review artifacts.
- Internal validation reports.

## Uses Not Allowed Without Proof

- Public or customer-facing claims that LiveSafe is backed by active EXOCHAIN
  runtime authority.
- Claims that root, FROST, settlement, ratification, custody, consent,
  provenance, or authority-chain enforcement is active for a LiveSafe feature.
- Claims that a user medical jacket, genetic import, P.A.C.E. obligation,
  emergency card, AI agent, marketplace template, or legacy capability has
  verified EXOCHAIN enforcement before the specific adapter and proof gate pass.
- Green verified visual treatment before completed proof gates for the exact
  claim.

## Implementation

`src/genesis-trust.ts` implements `evaluateGenesisTrust` for the TypeScript
surface policy, and `src/genesis_development_trust.rs` implements
`evaluate_genesis_development_trust` for the Rust domain contract. Both keep
internal genesis development allowed while denying external trust signaling
until the proof gates above pass. Rust coverage lives in
`tests/genesis_development_trust.rs`. `src/trust-signal.ts` implements the
visual trust-state token map.
