/**
 * Authority chain builder and validator.
 *
 * An authority chain is an ordered list of delegation links where each
 * `grantee` is the `grantor` of the next link. The chain terminates at a
 * specific actor. Mirrors the Rust SDK's `AuthorityChainBuilder` API.
 */
import type { Did } from '../types.js';
/** A single delegation link. */
export interface ChainLink {
    readonly grantor: Did;
    readonly grantee: Did;
    readonly permissions: readonly string[];
}
/** A fully validated authority chain. */
export interface ValidatedChain {
    readonly depth: number;
    readonly links: readonly ChainLink[];
    readonly terminal: Did;
}
/** Fluent builder for an authority chain. */
export declare class AuthorityChainBuilder {
    #private;
    /** Append a delegation link. Returns `this` for chaining. */
    addLink(grantor: Did | string, grantee: Did | string, permissions: readonly string[]): this;
    /**
     * Validate the chain topology and return a {@link ValidatedChain}.
     *
     * Rules:
     * - At least one link.
     * - For each consecutive pair, `links[i].grantee === links[i+1].grantor`.
     * - The final `grantee` must equal `terminalActor`.
     */
    build(terminalActor: Did | string): ValidatedChain;
}
//# sourceMappingURL=chain.d.ts.map