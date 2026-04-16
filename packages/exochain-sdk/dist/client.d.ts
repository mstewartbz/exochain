/**
 * High-level {@link ExochainClient} — the one-stop entry point for talking to
 * an `exo-gateway` instance. Each domain API (identity, consent, governance,
 * authority) wraps the shared {@link HttpTransport}.
 */
import { HttpTransport } from './transport/http.js';
import type { HealthResponse, Did, Hash256, QuorumResult } from './types.js';
/** Options for constructing an {@link ExochainClient}. */
export interface ExochainClientOptions {
    readonly baseUrl: string;
    readonly apiKey?: string;
    readonly timeout?: number;
    readonly fetch?: typeof fetch;
}
/** Identity-related gateway calls. */
export declare class IdentityApi {
    #private;
    constructor(http: HttpTransport);
    /** Resolve a DID to its DID document via `GET /identity/did/{did}`. */
    resolve(did: Did): Promise<unknown>;
    /** Register a DID document via `POST /identity/did`. */
    register(document: unknown): Promise<{
        did: Did;
    }>;
}
/** Consent / bailment gateway calls. */
export declare class ConsentApi {
    #private;
    constructor(http: HttpTransport);
    /** Submit a bailment proposal for processing. */
    proposeBailment(body: unknown): Promise<{
        proposalId: Hash256;
    }>;
    /** Fetch a bailment proposal by its content-addressed ID. */
    getBailment(proposalId: Hash256): Promise<unknown>;
}
/** Governance gateway calls. */
export declare class GovernanceApi {
    #private;
    constructor(http: HttpTransport);
    /** Create a decision via `POST /governance/decision`. */
    createDecision(body: unknown): Promise<{
        decisionId: Hash256;
    }>;
    /** Cast a vote on an existing decision. */
    castVote(decisionId: Hash256, body: unknown): Promise<void>;
    /** Fetch a decision's current state (including tallied quorum). */
    getDecision(decisionId: Hash256): Promise<{
        decisionId: Hash256;
        status: string;
        quorum?: QuorumResult;
    }>;
}
/** Authority chain gateway calls. */
export declare class AuthorityApi {
    #private;
    constructor(http: HttpTransport);
    /** Submit a validated authority chain for persistence. */
    submitChain(chain: unknown): Promise<{
        chainId: Hash256;
    }>;
    /** Fetch an authority chain by id. */
    getChain(chainId: Hash256): Promise<unknown>;
}
/** High-level client combining all domain APIs over a shared transport. */
export declare class ExochainClient {
    #private;
    readonly identity: IdentityApi;
    readonly consent: ConsentApi;
    readonly governance: GovernanceApi;
    readonly authority: AuthorityApi;
    constructor(opts: ExochainClientOptions);
    /** Gateway health probe. */
    health(): Promise<HealthResponse>;
}
//# sourceMappingURL=client.d.ts.map