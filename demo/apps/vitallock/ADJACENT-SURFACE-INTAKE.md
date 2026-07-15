# VitalLock Adjacent Surface Intake

VitalLock is not the canonical EXOCHAIN Rust trust fabric.

- Owner/accountable maintainer: Exochain Foundation / VitalLock maintainer
- Deployment status: `prototype`
- Constitutional trust claims: none. This React shell does not prove EXOCHAIN
  enforcement and cannot mint or simulate core outcomes.
- Core state access: none in the current browser build. Browser-local vault and
  messaging flows remain adjacent unless a tested core adapter is introduced.
- Exact trust boundary: all files under `demo/apps/vitallock` are proprietary
  adjacent code. Canonical Rust APIs and generated WASM bindings remain outside
  the app and are the only possible enforcement authorities.
- Test and CI gate: `npm --prefix demo/apps/vitallock ci`,
  `npm --prefix demo/apps/vitallock run test:security`,
  `npm --prefix demo/apps/vitallock run surface-policy:check`,
  `npm --prefix demo/apps/vitallock run build`, and
  `npm --prefix demo/apps/vitallock audit --audit-level=moderate`.
- Secrets and configuration: Vite environment and same-origin API proxy only.
  No core signing keys, bootstrap tokens, tenant secrets, authority material,
  or emergency-override credentials are permitted.
- Rollback/disablement: remove the hosting route or stop the app. Core state is
  unaffected.
- Licensing: proprietary; written terms from the Exochain Foundation are
  required for authorized use.
