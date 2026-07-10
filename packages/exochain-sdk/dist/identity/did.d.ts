import type { Did } from '../types.js';
/**
 * Validate a candidate DID string and return it branded as a {@link Did}.
 * Throws {@link IdentityError} if the input is not a well-formed `did:exo:`.
 */
export declare function validateDid(s: string): Did;
/** Type-guard form of {@link validateDid} that returns a boolean. */
export declare function isDid(s: string): s is Did;
//# sourceMappingURL=did.d.ts.map