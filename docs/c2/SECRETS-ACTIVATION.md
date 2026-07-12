<!--
Copyright 2026 Exochain Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
-->

# Presidential C2 — Secrets Activation (operator)

Cloud agents with the default GitHub App token **cannot** write Actions secrets
(`HTTP 403`). Railway variables require an authenticated Railway session owned
by Bob (or a scoped project token). Values are never committed.

## Prerequisites

1. PR #791 merged to `main` (or this branch deployed intentionally).
2. Machine dogfood bridge green (`presidential_c2_bridge`).
3. Human Bob + Max dual-gate on one live GAP/ratification item recorded in
   `DOGFOOD-REHEARSAL-*.md` as **Complete**.

## GitHub Actions secrets (`exochain/exochain`)

Run as a repo admin (Bob), not as the cloud agent:

```bash
# Slack incoming webhook for chairman CCIR push (primary)
gh secret set PRESIDENTIAL_SLACK_WEBHOOK_URL --body "$PRESIDENTIAL_SLACK_WEBHOOK_URL"

# Twilio auth token used only as SMS secondary when Slack fails / critical
gh secret set PRESIDENTIAL_TWILIO_AUTH_TOKEN --body "$PRESIDENTIAL_TWILIO_AUTH_TOKEN"

# Optional companions if CommandBase SMS path requires them in Actions:
# gh secret set PRESIDENTIAL_TWILIO_ACCOUNT_SID --body "$PRESIDENTIAL_TWILIO_ACCOUNT_SID"
# gh secret set PRESIDENTIAL_TWILIO_FROM --body "$PRESIDENTIAL_TWILIO_FROM"
# gh secret set PRESIDENTIAL_TWILIO_TO --body "$PRESIDENTIAL_TWILIO_TO"
```

Verify names only (values redacted):

```bash
gh secret list | rg 'PRESIDENTIAL_'
```

## Railway variables (project `372de75b-5f44-46c2-ab70-3c3185b5d81e`)

Target the EXOCHAIN node / CommandBase-adjacent service that hosts Presidential
Desk push adapters. Example for **development** first:

```bash
export PATH="$HOME/.local/node_modules/.bin:$PATH"
railway login --browserless   # or bare login on a machine with your browser

PROJECT=372de75b-5f44-46c2-ab70-3c3185b5d81e
ENV=3dc06fb6-c3df-4fe4-8807-0da0e62e4028          # development
# ENV=a223bc12-fbe4-430f-abce-8e3ee7c9abd3        # staging
# ENV=1e5153e1-15f4-4447-bf7c-029af33927fb        # production
NODE=4d8384d3-be5d-48d6-a914-97eb6133e53d         # exochain-node service id

printf '%s' "$EXOCHAIN_API_BASE_URL" | railway variable set EXOCHAIN_API_BASE_URL \
  --stdin --project "$PROJECT" --environment "$ENV" --service "$NODE"

printf '%s' "$PRESIDENTIAL_SLACK_WEBHOOK_URL" | railway variable set PRESIDENTIAL_SLACK_WEBHOOK_URL \
  --stdin --project "$PROJECT" --environment "$ENV" --service "$NODE"

printf '%s' "$PRESIDENTIAL_TWILIO_AUTH_TOKEN" | railway variable set PRESIDENTIAL_TWILIO_AUTH_TOKEN \
  --stdin --project "$PROJECT" --environment "$ENV" --service "$NODE"
```

Promote staging → production only after development dogfood close.

## Push enablement gate

Do **not** enable live Slack/SMS emission until:

1. Secrets present on GitHub + Railway (names above).
2. Dogfood rehearsal status is **Complete** (Bob + Max live dual attestation).
3. Explicit promote of the presidential workflow / desk adapter beyond
   fail-closed advisory mode.

Until then, `.github/workflows/presidential-daily-attention.yml` stays advisory
and prints that push remains disabled when secrets are absent.
