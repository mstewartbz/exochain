import { test } from 'node:test';
import { strictEqual, ok, rejects, throws } from 'node:assert/strict';
import { DecisionBuilder } from '../src/governance/decision.js';
import { Vote, VoteChoice } from '../src/governance/vote.js';
import { GovernanceError } from '../src/errors.js';
const PROPOSER = 'did:exo:proposer';
async function baseDecision() {
    return new DecisionBuilder({
        title: 'Fund proposal',
        description: 'Allocate budget',
        proposer: PROPOSER,
    }).build();
}
test('DecisionBuilder produces a decision in proposed state', async () => {
    const d = await baseDecision();
    strictEqual(d.title, 'Fund proposal');
    strictEqual(d.description, 'Allocate budget');
    strictEqual(d.status, 'proposed');
    strictEqual(d.votes.length, 0);
    strictEqual(d.decisionId.length, 64);
});
test('DecisionBuilder rejects empty title', async () => {
    await rejects(async () => new DecisionBuilder({ title: '', description: 'x', proposer: PROPOSER }).build(), GovernanceError);
});
test('DecisionBuilder optional class is surfaced', async () => {
    const d = await new DecisionBuilder({
        title: 't',
        description: 'd',
        proposer: PROPOSER,
    })
        .decisionClass('ordinary')
        .build();
    strictEqual(d.class, 'ordinary');
});
test('castVote appends to the decision', async () => {
    const d = await baseDecision();
    d.castVote(new Vote({ voter: 'did:exo:v1', choice: VoteChoice.Approve }));
    strictEqual(d.votes.length, 1);
    strictEqual(d.votes[0]?.choice, 'approve');
});
test('castVote rejects duplicate voter', async () => {
    const d = await baseDecision();
    d.castVote(new Vote({ voter: 'did:exo:v1', choice: VoteChoice.Approve }));
    throws(() => d.castVote(new Vote({ voter: 'did:exo:v1', choice: VoteChoice.Reject })), GovernanceError);
});
test('checkQuorum tallies approvals vs rejections vs abstentions', async () => {
    const d = await baseDecision();
    d.castVote(new Vote({ voter: 'did:exo:v1', choice: VoteChoice.Approve }));
    d.castVote(new Vote({ voter: 'did:exo:v2', choice: VoteChoice.Approve }));
    d.castVote(new Vote({ voter: 'did:exo:v3', choice: VoteChoice.Reject }));
    d.castVote(new Vote({ voter: 'did:exo:v4', choice: VoteChoice.Abstain }));
    const q = d.checkQuorum(2);
    ok(q.met);
    strictEqual(q.threshold, 2);
    strictEqual(q.totalVotes, 4);
    strictEqual(q.approvals, 2);
    strictEqual(q.rejections, 1);
    strictEqual(q.abstentions, 1);
});
test('checkQuorum reports not-met when below threshold', async () => {
    const d = await baseDecision();
    d.castVote(new Vote({ voter: 'did:exo:v1', choice: VoteChoice.Approve }));
    const q = d.checkQuorum(3);
    strictEqual(q.met, false);
    strictEqual(q.approvals, 1);
});
test('checkQuorum rejects invalid threshold', async () => {
    const d = await baseDecision();
    throws(() => d.checkQuorum(-1), GovernanceError);
    throws(() => d.checkQuorum(1.5), GovernanceError);
});
test('Vote withRationale returns a new vote with rationale', () => {
    const v = new Vote({ voter: 'did:exo:v', choice: VoteChoice.Reject }).withRationale('too risky');
    strictEqual(v.rationale, 'too risky');
});
test('Vote rejects invalid choice', () => {
    throws(() => new Vote({
        voter: 'did:exo:v',
        choice: 'maybe',
    }), GovernanceError);
});
test('Decision IDs are deterministic for identical inputs', async () => {
    const a = await baseDecision();
    const b = await baseDecision();
    strictEqual(a.decisionId, b.decisionId);
});
//# sourceMappingURL=governance.test.js.map