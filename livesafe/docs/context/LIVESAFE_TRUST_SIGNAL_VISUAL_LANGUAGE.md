# LiveSafe Trust Signal Visual Language

## Source Basis

- `AGENTS.md`
- `docs/TEST_PLAN.md`
- `docs/TRUST_SIGNAL_VISUAL_LANGUAGE.md`
- `docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md`
- `docs/context/LIVESAFE_GENESIS_DEVELOPMENT_TRUST.md`
- `context/canon/2026-05-25-phase-15-outward-trust-visual-language.md`
- `src/trust-signal.ts`
- `tests/trust-signal.test.ts`
- `tests/trust-signal-homologation.test.ts`

## Ground Truth

LiveSafe already has executable outward trust-signal policy in
`src/trust-signal.ts` plus focused tests for both visible output requirements
and homologation behavior. Repo truth today is:

- every trust-bearing output needs a visible `AVC` badge,
- trust state needs a lock-style or shield-style symbol, color treatment, glow
  treatment, human-readable status, and machine-readable state,
- `not-verified` output uses the exact display text `THIS IS NOT YET VERIFIED`,
- homologated output must preserve the canonical machine state and display
  meaning across jurisdiction, language, script, device, and holonic context,
  and
- green verified treatment is not available for public trust-bearing claims
  until proof, ceremony, and adapter gates pass.

This control document maps the current canon, written visual-language policy,
and executable contracts to the adjacent-surface LiveSafe trust-signal posture.
It does not activate EXOCHAIN-backed authority, root-backed trust, or public
verification claims.

## State Palette

The current executable trust-signal palette is:

| State | Color | Badge | Icon | Display text | Machine-readable state | External trust claim |
| --- | --- | --- | --- | --- | --- | --- |
| `not-verified` | red | `AVC` | `lock-open` | `THIS IS NOT YET VERIFIED` | `not_verified` | denied |
| `genesis-pending` | yellow | `AVC` | `lock-clock` | `GENESIS VERIFICATION PENDING` | `genesis_pending` | denied |
| `internal-proof` | blue | `AVC` | `shield-check` | `INTERNAL PROOF ONLY` | `internal_proof_only` | denied |
| `externally-verified` | green | `AVC` | `lock-check` | `VERIFIED` | `externally_verified` | allowed only when proof gates pass |

This palette is defined in `src/trust-signal.ts` and is the current source of
truth for visual tokens, display strings, CSS classes, glow classes, and
claim-eligibility state.

## Output Anatomy And CSS Contract

- Path classification: adjacent surface documentation and domain-contract
  mapping.
- `src/trust-signal.ts` currently defines:
  - the trust token map for red, yellow, blue, and green states,
  - visible output gates for `AVC`, icon, color, glow, human-readable text,
    machine-readable state, and accessible labels,
  - public surfaces `customer-portal`, `public-website`, `printed-card`, and
    `api-response`,
  - homologation requirements for jurisdiction, language, locale, writing
    system, assistive support, touch-target sizing, and stable layout.
- `tests/trust-signal.test.ts` proves:
  - trust-bearing public output requires the `AVC` badge,
  - trust-bearing public output requires lock or shield symbolism, color, glow,
    status text, machine-readable state, and accessibility labels,
  - `not-verified` output stays visually unmistakable,
  - green verified output is not accepted without the executable verified state.
- `tests/trust-signal-homologation.test.ts` proves:
  - supported jurisdictional, linguistic, ethnographic, device, and holonic
    modalities are explicit,
  - mobile and tablet trust controls require 44px minimum touch targets,
  - cultural-symbol review and non-color-only cues are required,
  - machine-state and display-meaning drift are denied,
  - stable layout is required across individual, family, P.A.C.E. network,
    responder, organization, and agent contexts,
  - Japanese `Jpan` output is allowed only when canonical trust meaning is
    preserved.

The current CSS contract remains:

- `trust-signal trust-signal--red trust-signal--not-verified`
- `trust-signal trust-signal--yellow trust-signal--genesis-pending`
- `trust-signal trust-signal--blue trust-signal--internal-proof`
- `trust-signal trust-signal--green trust-signal--externally-verified`
- `trust-glow trust-glow--red`
- `trust-glow trust-glow--yellow`
- `trust-glow trust-glow--blue`
- `trust-glow trust-glow--green`

## Surface And Homologation Requirements

Current written and executable repo truth requires trust-state output to stay
consistent across these surfaces:

- public website surfaces adjacent to trust-bearing copy,
- customer portal headers, cards, details, and confirmations,
- printed ICE card and packet trust-state areas,
- API responses that emit trust-state fields,
- private review and validation summaries.

Current homologation requirements remain fail-closed:

- jurisdiction, region, locale, and language mappings are required,
- supported script codes currently include `Latn`, `Jpan`, `Hans`, `Hant`,
  `Kana`, `Hang`, `Kore`, `Arab`, `Hebr`, `Cyrl`, `Deva`, `Grek`, and `Thai`,
- color alone is insufficient; icon, text, and machine-readable state must also
  preserve meaning,
- assistive-technology support is required for homologated output,
- touch targets below 44px fail mobile and tablet trust controls,
- layout must stay stable across holonic contexts.

These requirements keep trust meaning homologated rather than merely
translated. No locale, jurisdiction, or device variant may weaken the signal
or imply verified status ahead of evidence.

## Verified Green Gate

Green verified treatment remains blocked for public trust-bearing claims unless
the exact claim surface passes the production trust gates already defined in
`docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md` and
`docs/context/LIVESAFE_GENESIS_DEVELOPMENT_TRUST.md`:

1. completed internal proof,
2. completed 7-of-13 FROST ceremony evidence when relevant,
3. a verified runtime adapter for the specific claim,
4. fail-closed test evidence for deny, timeout, and unavailable cases,
5. raw sensitive data remaining off-chain and out of anchors or exported
   artifacts.

Until those gates pass, public and customer-facing trust state must remain
`not-verified`, `genesis-pending`, or `internal-proof`. Green verified
treatment remains blocked even though the token exists in the executable map.

## Disablement And Rollback

- Disablement path: keep public and customer-facing trust-state rendering in
  red, yellow, or blue states until exact proof and adapter evidence exists for
  the claim.
- Runtime rollback path: if any future route attempts to render green verified
  output without completed gates, deny that output before rendering,
  persistence, print generation, or external writes.
- Surface rollback path: if homologation evidence, accessible labels, cultural
  review, or stable layout proof becomes incomplete, revert the affected
  surface to a non-verified state immediately.
- Documentation rollback path: if token names, display text, or CSS classes
  drift in executable source, update this control document only after tests and
  source files are brought back into alignment.
