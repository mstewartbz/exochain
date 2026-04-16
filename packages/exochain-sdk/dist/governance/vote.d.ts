/**
 * Vote primitives for governance decisions.
 */
import type { Did, VoteChoiceValue } from '../types.js';
/** Enum-like namespace for vote choices. Also usable as a type alias. */
export declare const VoteChoice: {
    readonly Approve: VoteChoiceValue;
    readonly Reject: VoteChoiceValue;
    readonly Abstain: VoteChoiceValue;
};
/** A single vote cast by a voter on a decision. */
export declare class Vote {
    readonly voter: Did;
    readonly choice: VoteChoiceValue;
    readonly rationale?: string;
    constructor(args: {
        voter: Did | string;
        choice: VoteChoiceValue;
        rationale?: string;
    });
    /** Attach (or replace) a rationale, returning a new {@link Vote}. */
    withRationale(rationale: string): Vote;
}
/** Runtime type-guard for {@link VoteChoiceValue}. */
export declare function isVoteChoice(v: unknown): v is VoteChoiceValue;
//# sourceMappingURL=vote.d.ts.map