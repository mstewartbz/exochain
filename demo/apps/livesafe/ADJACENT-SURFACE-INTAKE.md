# LiveSafe Adjacent Surface Intake

LiveSafe is not the canonical EXOCHAIN Rust trust fabric.

- Owner/accountable maintainer: Exochain Foundation / LiveSafe maintainer
- Deployment status: `prototype`
- Constitutional trust claims: none. This React shell does not prove EXOCHAIN
  enforcement and cannot mint or simulate core outcomes.
- Core state access: none in the current browser build. Browser crypto and API
  calls are adjacent behavior unless a separately tested core adapter is used.
- Exact trust boundary: all files under `demo/apps/livesafe` are proprietary
  adjacent product code. Canonical Rust APIs and generated WASM bindings remain
  outside the app and are the only possible enforcement authorities.
- Test and CI gate: `npm --prefix demo/apps/livesafe ci`,
  `npm --prefix demo/apps/livesafe run surface-policy:check`,
  `npm --prefix demo/apps/livesafe run build`, and
  `npm --prefix demo/apps/livesafe audit --audit-level=moderate`.
- Secrets and configuration: Vite environment and same-origin API proxy only.
  No core signing keys, bootstrap tokens, tenant secrets, authority material,
  or emergency-override credentials are permitted.
- Rollback/disablement: remove the hosting route or stop the app. Core state is
  unaffected.
- Licensing: commercial terms are required through an active EXOCHAIN bailment
  licensure record and EXOCHAIN usage accounting.
