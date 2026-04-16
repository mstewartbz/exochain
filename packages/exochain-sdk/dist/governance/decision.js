/**
 * Governance decisions — build, cast votes, check quorum.
 *
 * Mirrors the Rust SDK's `governance` module. Decision IDs are content-
 * addressed (SHA-256 over the canonical title/description/proposer payload),
 * votes are appended in-order, and duplicate voters are rejected.
 */
import { GovernanceError } from '../errors.js';
import { validateDid } from '../identity/did.js';
import { sha256, bytesToHex } from '../crypto/hash.js';
/** A full governance decision with accumulated votes. */
export class Decision {
    decisionId;
    title;
    description;
    proposer;
    status;
    class;
    #votes;
    constructor(args) {
        this.decisionId = args.decisionId;
        this.title = args.title;
        this.description = args.description;
        this.proposer = args.proposer;
        this.status = args.status ?? 'proposed';
        if (args.class !== undefined) {
            this.class = args.class;
        }
        this.#votes = [];
    }
    /** Read-only snapshot of votes cast so far. */
    get votes() {
        return this.#votes;
    }
    /**
     * Append a vote. Throws {@link GovernanceError} if the voter has already
     * voted on this decision.
     */
    castVote(vote) {
        for (const existing of this.#votes) {
            if (existing.voter === vote.voter) {
                throw new GovernanceError(`voter ${vote.voter} has already cast a vote`);
            }
        }
        this.#votes.push(vote);
    }
    /**
     * Tally the votes and report whether the approval count meets `threshold`.
     */
    checkQuorum(threshold) {
        if (!Number.isInteger(threshold) || threshold < 0) {
            throw new GovernanceError('threshold must be a non-negative integer');
        }
        let approvals = 0;
        let rejections = 0;
        let abstentions = 0;
        for (const v of this.#votes) {
            if (v.choice === 'approve')
                approvals++;
            else if (v.choice === 'reject')
                rejections++;
            else
                abstentions++;
        }
        return {
            met: approvals >= threshold,
            threshold,
            totalVotes: this.#votes.length,
            approvals,
            rejections,
            abstentions,
        };
    }
}
/** Builder for a {@link Decision}. */
export class DecisionBuilder {
    #title;
    #description;
    #proposer;
    #class;
    constructor(args) {
        this.#title = args.title;
        this.#description = args.description;
        this.#proposer =
            typeof args.proposer === 'string' ? validateDid(args.proposer) : args.proposer;
    }
    /** Attach an optional decision class (free-form label). */
    decisionClass(name) {
        this.#class = name;
        return this;
    }
    /** Validate and build the {@link Decision}. */
    async build() {
        if (this.#title.length === 0) {
            throw new GovernanceError('title must be non-empty');
        }
        const decisionId = await computeDecisionId(this.#title, this.#description, this.#proposer);
        const init = {
            decisionId,
            title: this.#title,
            description: this.#description,
            proposer: this.#proposer,
        };
        if (this.#class !== undefined) {
            init.class = this.#class;
        }
        return new Decision(init);
    }
}
async function computeDecisionId(title, description, proposer) {
    const enc = new TextEncoder();
    const a = enc.encode(title);
    const b = enc.encode(description);
    const c = enc.encode(proposer);
    const payload = new Uint8Array(a.length + 1 + b.length + 1 + c.length);
    let off = 0;
    payload.set(a, off);
    off += a.length;
    payload[off++] = 0;
    payload.set(b, off);
    off += b.length;
    payload[off++] = 0;
    payload.set(c, off);
    const digest = await sha256(payload);
    return bytesToHex(digest);
}
//# sourceMappingURL=decision.js.map