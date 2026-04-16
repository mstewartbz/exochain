/**
 * DID validation.
 *
 * The fabric uses the `did:exo:<method-specific-id>` format. Validation here
 * is intentionally lightweight: we require the prefix, a non-empty method-
 * specific identifier, and a restricted character set. Stronger checks (e.g.
 * resolution, existence on the fabric) are server-side concerns.
 */
import { IdentityError } from '../errors.js';
/** Regex for the method-specific portion after `did:exo:`. */
const METHOD_SPECIFIC = /^[a-zA-Z0-9._-]+$/;
/**
 * Validate a candidate DID string and return it branded as a {@link Did}.
 * Throws {@link IdentityError} if the input is not a well-formed `did:exo:`.
 */
export function validateDid(s) {
    if (typeof s !== 'string') {
        throw new IdentityError('DID must be a string');
    }
    if (!s.startsWith('did:exo:')) {
        throw new IdentityError(`DID must start with "did:exo:" (got "${s}")`);
    }
    const method = s.slice('did:exo:'.length);
    if (method.length === 0) {
        throw new IdentityError('DID method-specific identifier is empty');
    }
    if (!METHOD_SPECIFIC.test(method)) {
        throw new IdentityError(`DID method-specific identifier contains invalid characters: "${method}"`);
    }
    return s;
}
/** Type-guard form of {@link validateDid} that returns a boolean. */
export function isDid(s) {
    try {
        validateDid(s);
        return true;
    }
    catch {
        return false;
    }
}
//# sourceMappingURL=did.js.map