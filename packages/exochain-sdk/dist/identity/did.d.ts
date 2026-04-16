/**
 * DID validation.
 *
 * The fabric uses the `did:exo:<method-specific-id>` format. Validation here
 * is intentionally lightweight: we require the prefix, a non-empty method-
 * specific identifier, and a restricted character set. Stronger checks (e.g.
 * resolution, existence on the fabric) are server-side concerns.
 */
import type { Did } from '../types.js';
/**
 * Validate a candidate DID string and return it branded as a {@link Did}.
 * Throws {@link IdentityError} if the input is not a well-formed `did:exo:`.
 */
export declare function validateDid(s: string): Did;
/** Type-guard form of {@link validateDid} that returns a boolean. */
export declare function isDid(s: string): s is Did;
//# sourceMappingURL=did.d.ts.map