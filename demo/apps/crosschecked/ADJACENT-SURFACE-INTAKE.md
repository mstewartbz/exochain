# CrossChecked Adjacent Surface Intake

CrossChecked is not the canonical EXOCHAIN Rust trust fabric.

- Owner/accountable maintainer: Exochain Foundation / CrossChecked maintainer
- Deployment status: `prototype`
- Constitutional trust claims: none. This React shell does not prove EXOCHAIN
  enforcement and cannot mint or simulate core outcomes.
- Core state access: none in the current browser build. API requests target the
  proprietary adjacent CrossChecked service, which has its own authenticated
  boundary before any separately configured core adapter.
- Exact trust boundary: all files under `demo/apps/crosschecked` are proprietary
  adjacent product code. Canonical Rust APIs and generated WASM bindings remain
  outside the app and are the only possible enforcement authorities.
- Test and CI gate: `npm --prefix demo/apps/crosschecked ci`,
  `npm --prefix demo/apps/crosschecked run surface-policy:check`,
  `npm --prefix demo/apps/crosschecked run build`, and
  `npm --prefix demo/apps/crosschecked audit --audit-level=moderate`.
- Secrets and configuration: Vite environment and same-origin API proxy only.
  No core signing keys, bootstrap tokens, tenant secrets, authority material,
  or emergency-override credentials are permitted.
- Rollback/disablement: remove the hosting route or stop the app. Core state is
  unaffected.
- Licensing: commercial terms are required through an active EXOCHAIN bailment
  licensure record and EXOCHAIN usage accounting.
