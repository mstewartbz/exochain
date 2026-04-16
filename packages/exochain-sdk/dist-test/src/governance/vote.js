/**
 * Vote primitives for governance decisions.
 */
import { GovernanceError } from '../errors.js';
import { validateDid } from '../identity/did.js';
/** Enum-like namespace for vote choices. Also usable as a type alias. */
export const VoteChoice = {
    Approve: 'approve',
    Reject: 'reject',
    Abstain: 'abstain',
};
/** A single vote cast by a voter on a decision. */
export class Vote {
    voter;
    choice;
    rationale;
    constructor(args) {
        this.voter = typeof args.voter === 'string' ? validateDid(args.voter) : args.voter;
        if (!isVoteChoice(args.choice)) {
            throw new GovernanceError(`invalid vote choice: ${String(args.choice)}`);
        }
        this.choice = args.choice;
        if (args.rationale !== undefined) {
            this.rationale = args.rationale;
        }
    }
    /** Attach (or replace) a rationale, returning a new {@link Vote}. */
    withRationale(rationale) {
        return new Vote({ voter: this.voter, choice: this.choice, rationale });
    }
}
/** Runtime type-guard for {@link VoteChoiceValue}. */
export function isVoteChoice(v) {
    return v === 'approve' || v === 'reject' || v === 'abstain';
}
//# sourceMappingURL=vote.js.map