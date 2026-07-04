# LiveSafe Trust Signal Visual Language

## Source Basis

- Bob Stewart current-thread direction on 2026-05-25: outward trust language
  should be viscerally symbolic and baked into output.
- Bob Stewart current-thread direction on 2026-05-25: any glance should tell the
  user when a claim is not yet verified.
- Bob Stewart current-thread examples: HTTPS-style lock, AVC badge, CSS glow
  treatment, and blue, red, yellow, green color states.

## Purpose

Trust state must be visible before a user reads explanatory copy. Every
trust-bearing LiveSafe output needs a symbolic badge, lock-style icon, color
treatment, glow treatment, human-readable status, and machine-readable state.

The visual system exists to prevent accidental overtrust during genesis and to
make verification status obvious in public websites, portals, printed cards,
API responses, and private review surfaces.

## State Palette

| State | Color | Badge | Icon | Display text | External trust claim |
| --- | --- | --- | --- | --- | --- |
| `not-verified` | red | AVC | lock-open | THIS IS NOT YET VERIFIED | denied |
| `genesis-pending` | yellow | AVC | lock-clock | GENESIS VERIFICATION PENDING | denied |
| `internal-proof` | blue | AVC | shield-check | INTERNAL PROOF ONLY | denied |
| `externally-verified` | green | AVC | lock-check | VERIFIED | allowed only when proof gates pass |

The executable token map lives in `src/trust-signal.ts`.

## Required Output Anatomy

Every trust-bearing output must include:

1. AVC badge.
2. Lock-style or shield-style symbol.
3. Colorized status treatment.
4. CSS glow treatment.
5. Human-readable status text.
6. Machine-readable status field.
7. Accessible label equivalent to the displayed state.

For `not-verified`, the red AVC badge, lock-open symbol, red glow, and exact
display text `THIS IS NOT YET VERIFIED` are mandatory.

## CSS Contract

The current class contract is:

- `trust-signal trust-signal--red trust-signal--not-verified`
- `trust-signal trust-signal--yellow trust-signal--genesis-pending`
- `trust-signal trust-signal--blue trust-signal--internal-proof`
- `trust-signal trust-signal--green trust-signal--externally-verified`
- `trust-glow trust-glow--red`
- `trust-glow trust-glow--yellow`
- `trust-glow trust-glow--blue`
- `trust-glow trust-glow--green`

Frontend implementation must keep stable dimensions for badges, icons, and
status labels so trust state changes do not shift layout.

## Surface Requirements

- Public website: badge appears adjacent to any trust-bearing claim.
- Customer portal: badge appears in headers, cards, detail views, and action
  confirmations that mention trust state.
- Printed ICE card and packet: badge appears near QR activation and trust-state
  copy.
- API response: machine state appears with any trust-state field.
- Private review: badge appears in reports and validation summaries.

## Homologation Requirements

Trust-state output must be homologated, not merely translated. The same trust
meaning must hold across jurisdiction, geography, language, writing system,
device, ethnographic context, and holonic context.

Required modalities:

- Jurisdictional: country, region, subdivision, legal regime.
- Geographic: region, locale, script, text direction.
- Linguistic: language, script, terminology, reading level.
- Ethnographic: plain language, cultural-symbol review, non-color-only status,
  assistive technology.
- Device: mobile, tablet, desktop, print, API.
- Holonic: individual, family, P.A.C.E. network, responder, organization,
  agent.

Supported script codes currently include:

- `Latn` for Latin scripts.
- `Jpan` for Japanese including Kanji and Kana presentation.
- `Hans` and `Hant` for Simplified and Traditional Han.
- `Kana`, `Hang`, `Kore`, `Arab`, `Hebr`, `Cyrl`, `Deva`, `Grek`, and `Thai`.

Every homologated output must preserve the canonical machine state and canonical
display meaning. Jurisdiction and language cannot weaken the signal. If a symbol
or color has different meaning in a target audience, that surface needs
cultural-symbol review and a non-color cue that preserves the state.

Mobile and tablet trust controls require at least 44px touch targets. Holonic
trust displays must keep stable layout across individual, family, P.A.C.E.,
responder, organization, and agent contexts.

## Anti-Overtrust Rule

Unverified, genesis-pending, and internal-proof states must never rely on subtle
copy alone. They must be visually different from verified output by color,
symbol, label, and machine-readable state.

Green verified treatment is reserved for completed internal proof, completed
7-of-13 FROST ceremony when relevant, and a verified runtime adapter for the
specific claim.

## Implementation

`src/trust-signal.ts` provides the trust signal token map and
`evaluateTrustSignalOutput`. The same module provides
`evaluateTrustSignalHomologation` for jurisdiction, language, script, device,
ethnographic, and holonic presentation. Tests live in
`tests/trust-signal.test.ts` and `tests/trust-signal-homologation.test.ts`.
