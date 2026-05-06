# EXOCHAIN — client SDK packages

This directory holds the non-Rust client SDKs and prebuilt WebAssembly
artifacts for the EXOCHAIN constitutional governance fabric. The authoritative
Rust implementation lives in [`../crates`](../crates); the packages here are
downstream-facing distributions tailored to browser, Node.js, and Python
environments.

## Index

| Package                                   | Language                 | Distribution                              | Purpose                                                                 |
| ----------------------------------------- | ------------------------ | ----------------------------------------- | ----------------------------------------------------------------------- |
| [`exochain-sdk`](./exochain-sdk/)         | TypeScript / JavaScript  | `npm install @exochain/sdk`               | Pure-JS client SDK for browsers and Node 20+; HTTP transport to gateway. |
| [`exochain-py`](./exochain-py/)           | Python 3.11+             | `pip install exochain`                    | Python client SDK with `httpx` async transport and pydantic v2 models. |
| [`exochain-wasm`](./exochain-wasm/)       | WebAssembly + TS shim    | `npm install exochain-wasm`               | Precompiled WASM build of the Rust governance engine for embedding.     |

The canonical Rust SDK lives at
[`../crates/exochain-sdk`](../crates/exochain-sdk/). All three client SDKs
expose the same five domains (identity, consent, governance, authority,
kernel) with the same builder-pattern ergonomics, translated into each
language's idioms.

## Which SDK should I use?

Pick by deployment target first, then by language preference:

- **Browser, React, or Next.js** — `@exochain/sdk`. Pure ESM, no native
  extensions, runs anywhere Web Crypto exposes Ed25519 (current Chromium,
  Safari 17+, Firefox 130+).
- **Node.js service or CLI** — `@exochain/sdk` on Node 20 or newer. Same
  package, same APIs as the browser SDK.
- **Python async service or notebook** — `exochain`. Ships with `httpx` for
  async HTTP, `cryptography` for Ed25519, and `blake3` for DID derivation.
  Python 3.11+.
- **Native Rust service or embedded kernel** — reach for
  [`../crates/exochain-sdk`](../crates/exochain-sdk/) directly. It is the
  reference implementation.
- **Browser or Node app that wants the full kernel in-process** —
  `exochain-wasm`. Runs the Rust governance engine as WebAssembly.

## Cross-language compatibility notes

All SDKs agree on the wire format. JSON objects produced by one SDK
deserialize cleanly in the others.

They agree on local DID derivation and wire format:

- Rust, TypeScript, and Python derive DIDs as the first 8 bytes of
  **BLAKE3(public_key_bytes)**.
- JSON objects produced by one SDK deserialize cleanly in the others.

They do not agree on every locally derived content ID: Rust uses BLAKE3 for
bailment and decision IDs, while TypeScript and Python keep SHA-256 for those
client-side IDs. Trust the gateway-returned IDs when a canonical Rust fabric
identifier is required.

## Versioning

Each package follows semver and is versioned independently. The underlying
fabric protocol is versioned separately — consult the EXOCHAIN specification
at the repository root for protocol-compatibility guarantees.

## License

Each package is licensed under **Apache-2.0**. See the top-level
[`LICENSE`](../LICENSE) for the full text.
