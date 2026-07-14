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

# CommandBase EXOCHAIN Economy Adapter Intake

- Owner/accountable maintainer: EXOCHAIN operator / CommandBase maintainer.
- Deployment status: internal cockpit adapter.
- Constitutional trust claims: CommandBase may display EXOCHAIN-recorded HonorGood and mission-economics objects only when responses come from the configured EXOCHAIN API.
- Core state access: read/write through `EXOCHAIN_API_BASE_URL` and optional bearer token only.
- Trust boundary: CommandBase never computes authoritative settlements, anchors, receipts, or legal effects. It forwards operator requests to EXOCHAIN and displays EXOCHAIN responses.
- Test command: `node --test command-base/app/lib/auth.security.test.js command-base/app/auth-bootstrap.test.js command-base/app/lib/presidential-desk.test.js`.
- Secrets inventory: `EXOCHAIN_API_BASE_URL`; optional `EXOCHAIN_API_TOKEN` and optional
  `EXOCHAIN_AUTH_SECRET` (required if WASM auth backend is unavailable); optional
  `COMMAND_BASE_AUTH_BOOTSTRAP_TOKEN` for non-loopback operator auth bootstrap;
  optional `PRESIDENTIAL_SLACK_WEBHOOK_URL` and `PRESIDENTIAL_TWILIO_AUTH_TOKEN` for
  Mission C2 Chairman push (never returned by status/brief routes). Tokens
  are not logged or returned by status routes.
- Rollback/disablement: unset `EXOCHAIN_API_BASE_URL` to force HonorGood cockpit and
  Presidential Desk actions to fail closed; unset presidential Slack/Twilio secrets
  to disable push adapters.

## Presidential Desk (Mission C2) intake addendum

- Surface path: `command-base/app/public/presidential-desk/`, `command-base/app/lib/presidential-desk.js`
- Deployment status: `prototype` / internal
- Allowed EXOCHAIN constitutional trust claims: **none** — desk is advisory C2 UX only
- Core state access: read brief facts only via configured `EXOCHAIN_API_BASE_URL`; no local mint of consent/authority/provenance
- Trust boundary: ratify/veto require authenticated principals; irreversible acts require Bob (`bob-stewart`) + Max (`mstewartbz`) dual gate in EXOCHAIN Decision Forum; Slack ack is not authority
- Test command: `node --test command-base/app/lib/presidential-desk.test.js`
- Dogfood gate: live Slack/SMS automation remains disabled until `docs/c2/DOGFOOD.md` rehearsal completes
- Rollback: unset `EXOCHAIN_API_BASE_URL` and presidential push secrets; remove `/presidential-desk` route mount if needed
