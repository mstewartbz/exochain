# LiveSafe Proprietary IP Handling

## Source Basis

- Bob Stewart current-thread direction on 2026-05-24: "this is our IP."
- Current repository: `/Users/bobstewart/dev/livesafe`.
- Current baseline includes imported context for LiveSafe, VitalLock,
  InCaseOfEmergencyCard, Ambient, EXOCHAIN, `exo-legacy`, AI help and feedback,
  enterprise onboarding, P.A.C.E., medical jacket custody,
  content-addressed storage, and marketplace entitlements.

## Classification

The architecture, transfer packages, prompts, requirements, product lineage,
data-boundary model, onboarding mechanics, P.A.C.E. social-contract mechanics,
content-addressed storage entitlement model, ICE card packet design, and
LiveSafe commercial architecture are proprietary LiveSafe and EXOCHAIN project
IP unless Bob explicitly classifies a specific artifact for public release.

Public-domain civic source material is not proprietary IP. The U.S.
Constitution Preamble source phrase is "We the People." The "of the people, by
the people, for the people" civic formula is from the Gettysburg Address, not
the Constitution text. LiveSafe may use these as cited civic grounding, while
the LiveSafe interpretation, implementation, architecture, requirements,
contracts, workflows, tests, and product doctrine remain proprietary project IP.

## Proprietary Asset Register

The executable asset identifiers live in `src/ip-boundary.ts`:

- `exo-legacy-transfer-package`
- `ice-card-foldable-packet-concept`
- `pace-social-contract-onboarding`
- `medical-jacket-custody-model`
- `phenotypical-genotypical-data-classification`
- `content-addressed-storage-entitlement-model`
- `ai-help-feedback-agent-system`
- `marketplace-template-entitlement-model`
- `frontline-free-family-plan-eligibility`

## Handling Rules

- Keep detailed architecture, transfer artifacts, implementation prompts, and
  source-backed requirements inside private repositories or controlled agent
  sessions.
- Public materials may use only owner-approved summaries with proprietary
  detail removed.
- Preserve source provenance for every imported assistant output, research
  package, repo excerpt, image, PDF, meeting artifact, and user direction.
- Do not move raw personal, medical, genetic, emergency, contact, payment,
  credential, authority-chain, or operational data through IP artifacts.
- Do not publish detailed LiveSafe architecture, card packet mechanics,
  P.A.C.E. onboarding mechanics, `exo-legacy` transfer details, or agent-system
  implementation prompts to public repositories, public issue trackers, or
  public websites without explicit artifact-level approval.
- Treat EXOCHAIN core evidence as dependency evidence unless a verified adapter
  exists and tests prove the runtime path.

## Public Release Gate

Before any public release of LiveSafe architecture material, tests and review
must prove:

1. The artifact is classified as an approved public summary.
2. The artifact has source provenance.
3. Detailed transfer packages and implementation prompts are absent.
4. Raw sensitive data and operational secrets are absent.
5. EXOCHAIN trust language matches verified runtime evidence.
6. Bob has approved the exact artifact for public release.

## Civic Source Gate

Before LiveSafe uses constitutional or civic language in product copy,
architecture doctrine, policy, or runtime authority descriptions, review must
prove:

1. The exact source is cited.
2. U.S. Constitution text is not blended with later civic doctrine.
3. Public-domain civic source text is not classified as proprietary IP.
4. LiveSafe does not imply governmental authority, state action, or official
   public-office status.
5. Legal, custody, consent, and authority claims are grounded in verified code,
   policy, contract, or law, not civic language alone.

## Implementation

`src/ip-boundary.ts` provides the local decision function for IP movement:
`evaluateIpDisclosure`. It blocks proprietary internal artifacts and private
source evidence from public targets, blocks sensitive operational data in IP
artifacts, and requires source provenance. The same module provides
`evaluateCivicSourceUse`, which enforces the civic-source separation above.
