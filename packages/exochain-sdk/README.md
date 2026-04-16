# @exochain/sdk

TypeScript/JavaScript SDK for **EXOCHAIN** — the constitutional governance
fabric for AI systems and data sovereignty. EXOCHAIN provides cryptographically
verifiable identity (DID), consent, authority delegation, and governance
primitives that enforce policy *before* action rather than auditing it after.

This package is the pure-TypeScript, zero-runtime-dependency client. It exposes
the same ergonomic builder surface as the Rust `exochain-sdk` crate and adds an
HTTP client for talking to a running `exo-gateway`.

## Installation

```bash
npm install @exochain/sdk
```

Requires **Node.js 20 or newer** (for stable Web Crypto Ed25519 support). The
package is ESM-only and ships with TypeScript declarations.

## Quick start

### Create an identity

```ts
import { Identity } from '@exochain/sdk';

const alice = await Identity.generate('alice');
console.log(alice.did);          // did:exo:<16 hex chars>
console.log(alice.publicKeyHex); // 64 hex chars (32 bytes)

const msg = new TextEncoder().encode('hello');
const sig = await alice.sign(msg);

const ok = await Identity.verify(alice.publicKeyHex, msg, sig);
```

### Propose a bailment (scoped consent)

```ts
import { BailmentBuilder } from '@exochain/sdk/consent';

const proposal = await new BailmentBuilder(alice.did, bob.did)
  .scope('data:medical')
  .durationHours(24)
  .build();

// proposal.proposalId is a deterministic, content-addressed hash.
```

### Create a governance decision and cast votes

```ts
import { DecisionBuilder, Vote, VoteChoice } from '@exochain/sdk/governance';

const decision = await new DecisionBuilder({
  title: 'Ratify change X',
  description: '...',
  proposer: alice.did,
}).build();

decision.castVote(new Vote({ voter: bob.did, choice: VoteChoice.Approve }));
decision.castVote(new Vote({ voter: carol.did, choice: VoteChoice.Approve }));

const quorum = decision.checkQuorum(2);
// { met: true, threshold: 2, approvals: 2, ... }
```

### Build and verify an authority chain

```ts
import { AuthorityChainBuilder } from '@exochain/sdk/authority';

const chain = new AuthorityChainBuilder()
  .addLink(root.did, mid.did, ['read'])
  .addLink(mid.did, leaf.did, ['read'])
  .build(leaf.did); // throws if the chain is broken
```

### Connect to an exo-gateway node

```ts
import { ExochainClient } from '@exochain/sdk';

const client = new ExochainClient({
  baseUrl: 'https://gateway.example.com',
  apiKey: process.env.EXOCHAIN_API_KEY,
});

const health = await client.health();
const resolved = await client.identity.resolve(alice.did);
```

## API reference

Each domain is available either from the package root or from a named subpath
export:

| Subpath                   | Exports                                              |
| ------------------------- | ---------------------------------------------------- |
| `@exochain/sdk`           | Everything — `ExochainClient`, all domain primitives |
| `@exochain/sdk/identity`  | `Identity`, `validateDid`, `isDid`, `deriveDid`      |
| `@exochain/sdk/consent`   | `BailmentBuilder`, `BailmentProposal`                |
| `@exochain/sdk/governance`| `Decision`, `DecisionBuilder`, `Vote`, `VoteChoice`  |
| `@exochain/sdk/authority` | `AuthorityChainBuilder`, `ChainLink`, `ValidatedChain` |
| `@exochain/sdk/crypto`    | `sha256`, `sha256Hex`, `sha256Hash`, hex helpers     |

### Branded types

`Did` and `Hash256` are branded string types. Use `validateDid(s)` at the
boundary of untrusted input — plain strings cannot be assigned to parameters
that expect a `Did`.

### Errors

All SDK errors extend `ExochainError`:

- `IdentityError`, `ConsentError`, `GovernanceError`, `AuthorityError`
- `KernelError` (reserved for the constitutional kernel)
- `CryptoError`, `TransportError`

Discriminate with `instanceof` rather than string matching.

## DID derivation — important note

This pure-JS SDK derives DIDs as:

```
did:exo: + first 16 hex chars of SHA-256(raw public key bytes)
```

The Rust SDK uses **BLAKE3** instead of SHA-256, because BLAKE3 is not available
in the Web Crypto API. DIDs produced by this SDK will **not** match DIDs
produced by the Rust SDK for the same keypair. For applications that need to
interoperate with the Rust fabric at the DID level, obtain the canonical DID
from the fabric and construct the identity via `Identity.fromKeypair`.

## Relation to the MCP server

The EXOCHAIN stack also ships an MCP (Model Context Protocol) server at
`exo-node` for use with AI agents. The SDK in this package targets application
developers writing TypeScript clients that speak directly to the gateway; the
MCP server is the bridge for AI agents that want constitutional governance as
tool-use.

## License

Apache-2.0 — see `LICENSE` at the repository root.
