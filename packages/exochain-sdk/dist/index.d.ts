/**
 * @exochain/sdk — TypeScript SDK for the EXOCHAIN constitutional governance fabric.
 *
 * The package entry point re-exports everything needed to build applications
 * against an `exo-gateway` or to use the pure-JS primitives (identity, consent,
 * governance, authority) in isolation.
 */
export * from './types.js';
export * from './errors.js';
export * from './client.js';
export { Identity, deriveDid } from './identity/keypair.js';
export { validateDid, isDid } from './identity/did.js';
export { BailmentBuilder } from './consent/bailment.js';
export type { BailmentProposal } from './consent/bailment.js';
export { Decision, DecisionBuilder } from './governance/decision.js';
export type { DecisionStatus } from './governance/decision.js';
export { Vote, VoteChoice, isVoteChoice } from './governance/vote.js';
export { AuthorityChainBuilder } from './authority/chain.js';
export type { ChainLink, ValidatedChain } from './authority/chain.js';
export { sha256, sha256Hex, sha256Hash, bytesToHex, hexToBytes } from './crypto/hash.js';
export { HttpTransport } from './transport/http.js';
export type { HttpTransportOptions } from './transport/http.js';
//# sourceMappingURL=index.d.ts.map