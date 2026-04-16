/**
 * Consent bailment builder.
 *
 * A bailment represents scoped, time-bounded consent from a bailor to a
 * bailee. `BailmentBuilder` mirrors the Rust SDK's builder pattern and
 * produces a {@link BailmentProposal} whose `proposalId` is a content-
 * addressed SHA-256 over the canonical fields.
 */
import type { Did, Hash256 } from '../types.js';
/** A validated bailment proposal. */
export interface BailmentProposal {
    readonly proposalId: Hash256;
    readonly bailor: Did;
    readonly bailee: Did;
    readonly scope: string;
    readonly durationHours: number;
    readonly createdAt: number;
}
/** Builder for a {@link BailmentProposal}. */
export declare class BailmentBuilder {
    #private;
    constructor(bailor: Did | string, bailee: Did | string);
    /** Set the scope string (e.g. `"data:medical"`). */
    scope(scope: string): this;
    /** Set the bailment duration in whole hours. */
    durationHours(hours: number): this;
    /** Validate the builder state and produce a {@link BailmentProposal}. */
    build(): Promise<BailmentProposal>;
}
//# sourceMappingURL=bailment.d.ts.map