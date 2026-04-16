/**
 * Governance decisions — build, cast votes, check quorum.
 *
 * Mirrors the Rust SDK's `governance` module. Decision IDs are content-
 * addressed (SHA-256 over the canonical title/description/proposer payload),
 * votes are appended in-order, and duplicate voters are rejected.
 */
import type { Did, Hash256, QuorumResult } from '../types.js';
import { Vote } from './vote.js';
/** Lifecycle states a decision may be in. */
export type DecisionStatus = 'proposed' | 'deliberating' | 'approved' | 'rejected' | 'challenged';
/** A full governance decision with accumulated votes. */
export declare class Decision {
    #private;
    readonly decisionId: Hash256;
    readonly title: string;
    readonly description: string;
    readonly proposer: Did;
    status: DecisionStatus;
    readonly class?: string;
    constructor(args: {
        decisionId: Hash256;
        title: string;
        description: string;
        proposer: Did;
        status?: DecisionStatus;
        class?: string;
    });
    /** Read-only snapshot of votes cast so far. */
    get votes(): readonly Vote[];
    /**
     * Append a vote. Throws {@link GovernanceError} if the voter has already
     * voted on this decision.
     */
    castVote(vote: Vote): void;
    /**
     * Tally the votes and report whether the approval count meets `threshold`.
     */
    checkQuorum(threshold: number): QuorumResult;
}
/** Builder for a {@link Decision}. */
export declare class DecisionBuilder {
    #private;
    constructor(args: {
        title: string;
        description: string;
        proposer: Did | string;
    });
    /** Attach an optional decision class (free-form label). */
    decisionClass(name: string): this;
    /** Validate and build the {@link Decision}. */
    build(): Promise<Decision>;
}
//# sourceMappingURL=decision.d.ts.map