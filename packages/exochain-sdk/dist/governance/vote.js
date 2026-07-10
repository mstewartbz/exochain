// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0
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