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

# CommandBase EXOCHAIN Adjacent Surface Intake

- Owner/accountable maintainer: EXOCHAIN operator / CommandBase maintainer.
- Deployment status: `internal` cockpit adapter.
- Constitutional trust claims: CommandBase may display EXOCHAIN-recorded HonorGood and mission-economics objects only when responses come from the configured EXOCHAIN API. CommandBase-local governance receipts, review-panel votes, and heuristic invariant checks are adjacent audit records and are not EXOCHAIN constitutional-kernel enforcement.
- Core state access: read/write through `EXOCHAIN_API_BASE_URL` and optional bearer token only.
- Trust boundary: CommandBase never computes authoritative settlements, anchors, EXOCHAIN receipts, governance outcomes, consent decisions, authority chains, or legal effects. It forwards operator requests to EXOCHAIN and displays EXOCHAIN responses. CommandBase-local heuristic checks may write local audit-trail records, but they never extend a trusted EXOCHAIN receipt hash chain.
- Test and CI gate: `cd command-base/app && npm ci && npm test && npm run audit:check`; from the repository root, `bash tools/test_commandbase_release_hardening.sh` is enforced by Gate 9.
- Secrets inventory: `EXOCHAIN_API_BASE_URL`; optional `EXOCHAIN_API_TOKEN` and optional
  `EXOCHAIN_AUTH_SECRET` (at least 32 bytes and required if the WASM auth backend is unavailable);
  `COMMANDBASE_WEBHOOK_SECRET` (at least 32 bytes and required before inbound SMS or Slack
  webhooks can authenticate); optional
  `COMMAND_BASE_AUTH_BOOTSTRAP_TOKEN` for non-loopback operator auth bootstrap;
  optional `PRESIDENTIAL_SLACK_WEBHOOK_URL` and `PRESIDENTIAL_TWILIO_AUTH_TOKEN` for
  Mission C2 Chairman push (never returned by status/brief routes). Tokens
  are not logged or returned by status routes. CommandBase does not share EXOCHAIN core
  signing keys, bootstrap credentials, tenant secrets, or emergency-override credentials.
- Rollback/disablement: unset `EXOCHAIN_API_BASE_URL` to force HonorGood cockpit and
  Presidential Desk actions to fail closed; stop the CommandBase process or unmount the
  adjacent routes to disable the surface. Unsetting `COMMANDBASE_WEBHOOK_SECRET` causes
  inbound webhooks to fail closed; unsetting presidential Slack/Twilio secrets disables
  push adapters. Disable CommandBase governance automation routes if local audit records
  are ever mistaken for EXOCHAIN core receipts.

## Presidential Desk (Mission C2) intake addendum

- Surface path: `command-base/app/public/presidential-desk/`, `command-base/app/lib/presidential-desk.js`
- Deployment status: `prototype` / internal
- Allowed EXOCHAIN constitutional trust claims: **none** — desk is advisory C2 UX only
- Core state access: read brief facts only via configured `EXOCHAIN_API_BASE_URL`; no local mint of consent/authority/provenance
- Trust boundary: ratify/veto require authenticated principals; irreversible acts require Bob (`bob-stewart`) + Max (`mstewartbz`) dual gate in EXOCHAIN Decision Forum; Slack ack is not authority
- Test command: `node --test command-base/app/lib/presidential-desk.test.js`
- Dogfood gate: live Slack/SMS automation remains disabled until `docs/c2/DOGFOOD.md` rehearsal completes
- Rollback: unset `EXOCHAIN_API_BASE_URL` and presidential push secrets; remove `/presidential-desk` route mount if needed
