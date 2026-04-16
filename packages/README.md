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
| [`exochain-py`](./exochain-py/)           | Python 3.11+             | `pip install exochain`                    | Pure-Python client SDK with `httpx` async transport and pydantic v2 models. |
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
  async HTTP and `cryptography` for Ed25519/SHA-256. Python 3.11+.
- **Native Rust service or embedded kernel** — reach for
  [`../crates/exochain-sdk`](../crates/exochain-sdk/) directly. It is the
  reference implementation and the only SDK that uses BLAKE3 natively.
- **Browser or Node app that wants the full kernel in-process** —
  `exochain-wasm`. Runs the Rust governance engine as WebAssembly.

## Cross-language compatibility notes

All SDKs agree on the wire format. JSON objects produced by one SDK
deserialize cleanly in the others.

They **do not** agree on locally derived hash digests:

- The Rust SDK derives DIDs, bailment IDs, and decision IDs using **BLAKE3**.
- The TypeScript and Python SDKs use **SHA-256** because Web Crypto does not
  ship BLAKE3.

For canonical DIDs across all three SDKs, resolve the DID from the fabric
(via `exo-gateway`) rather than deriving it locally, and construct the
identity with the language's `fromKeypair` / `from_keypair` constructor.

## Versioning

Each package follows semver and is versioned independently. The underlying
fabric protocol is versioned separately — consult the EXOCHAIN specification
at the repository root for protocol-compatibility guarantees.

## License

Each package is licensed under **Apache-2.0**. See the top-level
[`LICENSE`](../LICENSE) for the full text.
