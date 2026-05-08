# CommandBase EXOCHAIN Economy Adapter Intake

- Owner/accountable maintainer: EXOCHAIN operator / CommandBase maintainer.
- Deployment status: internal cockpit adapter.
- Constitutional trust claims: CommandBase may display EXOCHAIN-recorded HonorGood and mission-economics objects only when responses come from the configured EXOCHAIN API.
- Core state access: read/write through `EXOCHAIN_API_BASE_URL` and optional bearer token only.
- Trust boundary: CommandBase never computes authoritative settlements, anchors, receipts, or legal effects. It forwards operator requests to EXOCHAIN and displays EXOCHAIN responses.
- Test command: `node --test command-base/app/services/honorgood-economy.test.js`.
- Secrets inventory: `EXOCHAIN_API_BASE_URL`; optional `EXOCHAIN_API_TOKEN`. Tokens are not logged or returned by status routes.
- Rollback/disablement: unset `EXOCHAIN_API_BASE_URL` to force HonorGood cockpit actions to fail closed.
