# VitalLock Adjacent Surface Intake

Path classification: `demo/apps/vitallock` is an adjacent surface. It is not
EXOCHAIN core and it does not prove constitutional enforcement by proximity.

Owner and accountable maintainer: EXOCHAIN demo owner, accountable maintainer
Bob Stewart until a separate product owner is assigned.

Deployment status: `prototype`.

Constitutional trust claims: not allowed. VitalLock may describe browser-local
encryption behavior, but it must not claim EXOCHAIN constitutional protection
unless a tested core adapter path is added.

Core state access: the browser app does not read or write EXOCHAIN governance
state, consent records, authority chains, credentials, provenance records, or
constitutional outcomes. Messaging calls are adjacent demo flows.

Trust boundary: identity and private messaging key material remain browser-local.
The API may receive public identifiers, public encryption keys, and encrypted
envelopes only. The server must not receive passphrases, passphrase hashes,
private signing keys, or X25519 private keys.

Surface test command and CI gate: run `npm --prefix demo/apps/vitallock run
test:security` for the adjacent app source guard. Run `npm --prefix demo test
-- --project services --runInBand` or the focused VitalLock API vitest command
for service-boundary changes when the demo workspace dependencies are installed.

Secrets inventory and runtime configuration source: no production secrets are
required by the browser app. The local identity vault is stored in browser
`localStorage` encrypted with PBKDF2-SHA256 and AES-GCM from the user
passphrase. API connectivity is supplied by the Vite `/api` proxy in development
or by the deployed frontend origin in production.

Rollback or disablement path: remove the frontend route or disable the
`/api/messages/compose`, `/api/profile`, and `/api/messages/*` routes if the
surface leaks private material, misroutes encrypted envelopes, or overstates
EXOCHAIN core trust decisions.
