/**
 * Vote primitives for governance decisions.
 */

import { GovernanceError } from '../errors.js';
import type { Did, VoteChoiceValue } from '../types.js';
import { validateDid } from '../identity/did.js';

/** Enum-like namespace for vote choices. Also usable as a type alias. */
export const VoteChoice = {
  Approve: 'approve' as VoteChoiceValue,
  Reject: 'reject' as VoteChoiceValue,
  Abstain: 'abstain' as VoteChoiceValue,
} as const;

/** A single vote cast by a voter on a decision. */
export class Vote {
  public readonly voter: Did;
  public readonly choice: VoteChoiceValue;
  public readonly rationale?: string;

  constructor(args: { voter: Did | string; choice: VoteChoiceValue; rationale?: string }) {
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
  public withRationale(rationale: string): Vote {
    return new Vote({ voter: this.voter, choice: this.choice, rationale });
  }
}

/** Runtime type-guard for {@link VoteChoiceValue}. */
export function isVoteChoice(v: unknown): v is VoteChoiceValue {
  return v === 'approve' || v === 'reject' || v === 'abstain';
}
