# LiveSafe Demo Adjacent-Surface Intake

## Classification and ownership

- Path classification: proprietary adjacent surface.
- Product: LiveSafe.
- Owner: Exochain Foundation.
- Accountable maintainer: Bob Stewart.
- Deployment status: prototype. A Railway manifest exists, but this record does
  not assert that any deployment or public route is currently active.
- License: commercial terms required. This surface is `UNLICENSED` for package
  metadata purposes and is governed by `LICENSE` in this directory.

## Constitutional trust-claim boundary

- This surface is not allowed to claim EXOCHAIN constitutional enforcement.
- It is a static browser client for an adjacent `/api` service. Its development
  proxy does not call an EXOCHAIN core API directly and cannot mint consent,
  authority, provenance, governance, licensure, or usage-accounting outcomes.
- Any backend integration remains outside the browser trust boundary. A backend
  response is not an EXOCHAIN decision unless a separately tested core runtime
  adapter verifies and returns it.
- Product use requires commercial terms tracked through an EXOCHAIN
  `Licensure` bailment, `exo-economy-use-event-v1` accounting, and settlement.

## Core-state and data access

- Direct EXOCHAIN core reads: none.
- Direct EXOCHAIN core writes: none.
- Adjacent reads and writes: the browser calls relative `/api` routes for
  profiles, emergency plans, trustees, key generation, and related LiveSafe
  state. The Vite development proxy targets `http://localhost:3011`.
- Sensitive boundary: `/api/keys/generate` returns a LiveSafe keypair to the
  browser. The client stores the secret locally; it must never be treated as or
  share scope with an EXOCHAIN bootstrap, signing, tenant, or emergency-override
  credential.

## Validation and CI

- Install/build: `cd demo/apps/livesafe && npm ci && npm run build`.
- Boundary guard: `bash tools/test_livesafe_demo_security.sh`.
- CI gate: Gate 9 runs the boundary guard.
- Host-header enforcement: Vite preview accepts only localhost, loopback,
  Railway health checks, and Railway subdomains.

## Secrets and runtime configuration

- Static-build secrets: none permitted. No secret may be placed in a Vite
  client environment variable or committed source.
- Browser-held material: the adjacent API-generated LiveSafe keypair described
  above; stored client-side and scoped only to this product.
- Runtime source: Railway supplies `PORT`; Vite preview configuration supplies
  the host allowlist. The local development API target is fixed to loopback.

## Disablement and rollback

- Disablement: remove or disable the Railway service/public route and stop the
  Vite preview process.
- Code rollback: revert the product deployment to the last validated commit.
- Trust-claim rollback: remove the surface from public routing. Do not replace a
  failed adapter or host check with cached, simulated, or locally minted trust
  output.
