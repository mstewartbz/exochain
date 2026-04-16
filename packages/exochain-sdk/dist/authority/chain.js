/**
 * Authority chain builder and validator.
 *
 * An authority chain is an ordered list of delegation links where each
 * `grantee` is the `grantor` of the next link. The chain terminates at a
 * specific actor. Mirrors the Rust SDK's `AuthorityChainBuilder` API.
 */
import { AuthorityError } from '../errors.js';
import { validateDid } from '../identity/did.js';
/** Fluent builder for an authority chain. */
export class AuthorityChainBuilder {
    #links = [];
    /** Append a delegation link. Returns `this` for chaining. */
    addLink(grantor, grantee, permissions) {
        const grantorDid = typeof grantor === 'string' ? validateDid(grantor) : grantor;
        const granteeDid = typeof grantee === 'string' ? validateDid(grantee) : grantee;
        this.#links.push({
            grantor: grantorDid,
            grantee: granteeDid,
            permissions: [...permissions],
        });
        return this;
    }
    /**
     * Validate the chain topology and return a {@link ValidatedChain}.
     *
     * Rules:
     * - At least one link.
     * - For each consecutive pair, `links[i].grantee === links[i+1].grantor`.
     * - The final `grantee` must equal `terminalActor`.
     */
    build(terminalActor) {
        const terminal = typeof terminalActor === 'string' ? validateDid(terminalActor) : terminalActor;
        if (this.#links.length === 0) {
            throw new AuthorityError('authority chain is empty');
        }
        for (let i = 0; i < this.#links.length - 1; i++) {
            const a = this.#links[i];
            const b = this.#links[i + 1];
            if (a === undefined || b === undefined)
                continue;
            if (a.grantee !== b.grantor) {
                throw new AuthorityError(`broken delegation at index ${i}: ${a.grantee} !== ${b.grantor}`);
            }
        }
        const last = this.#links[this.#links.length - 1];
        if (last === undefined) {
            throw new AuthorityError('authority chain is empty');
        }
        if (last.grantee !== terminal) {
            throw new AuthorityError(`terminal mismatch: chain ends at ${last.grantee} but terminal is ${terminal}`);
        }
        return {
            depth: this.#links.length,
            links: [...this.#links],
            terminal,
        };
    }
}
//# sourceMappingURL=chain.js.map