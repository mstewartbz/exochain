# ExoForge EXOCHAIN Surface Intake

## Classification

ExoForge is an adjacent surface. It is an implementation factory and proposal
automation tool. It is not EXOCHAIN core, and it is not a settlement authority.

## Owner And Status

- Owner: EXOCHAIN maintainers
- Release status: internal
- Constitutional trust claims: allowed only when displaying or submitting
  responses returned by EXOCHAIN core APIs

## Core Access

The HonorGood adapter can submit complete economy payloads to:

- `POST /api/v1/economy/missions`
- `POST /api/v1/economy/contribution-receipts`
- `POST /api/v1/economy/legacy-receipts`
- `POST /api/v1/economy/rulesets`
- `POST /api/v1/economy/contribution-nodes`
- `POST /api/v1/economy/contribution-offers`

ExoForge may generate unratified LegacyReceipt proposals. EXOCHAIN core validates,
hashes, anchors, and records any submitted object.

## Trust Boundary

ExoForge can propose. EXOCHAIN records. ExoForge must not simulate authoritative
settlement, ratification, materiality decisions, consent, authority, or provenance.
Archon, Paperclip, and other upstream proposals remain unratified unless EXOCHAIN
core receives valid contributor acceptance and human ratification fields.

## Secrets And Runtime Configuration

- `EXOCHAIN_API_BASE_URL`: required to submit to EXOCHAIN core
- `EXOCHAIN_API_TOKEN`: optional bearer token

The adapter fails closed when `EXOCHAIN_API_BASE_URL` is absent. It must not print
tokens or environment variables.

## Tests And Gate

Run:

```bash
node --test exoforge/test/honorgood.test.js
```

The test gate proves that proposal output is unratified, submission goes to the
EXOCHAIN economy API, and the adapter has no local settlement authority.

## Rollback

Remove the `exoforge-honorgood` bin entry or unset `EXOCHAIN_API_BASE_URL` to
disable EXOCHAIN submission from ExoForge.
