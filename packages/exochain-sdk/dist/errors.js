/**
 * Typed error hierarchy for the EXOCHAIN SDK.
 *
 * All SDK errors extend {@link ExochainError}, which in turn extends the
 * standard `Error` class. Each subtype is thrown by the corresponding domain
 * module so callers can discriminate on `instanceof` without string matching.
 */
/** Base class for all SDK errors. */
export class ExochainError extends Error {
    name = 'ExochainError';
    constructor(message, options) {
        super(message, options);
        // Preserve prototype chain across targets that down-compile class inheritance.
        Object.setPrototypeOf(this, new.target.prototype);
    }
}
/** Identity / DID validation and keypair errors. */
export class IdentityError extends ExochainError {
    name = 'IdentityError';
}
/** Consent / bailment builder and proposal errors. */
export class ConsentError extends ExochainError {
    name = 'ConsentError';
}
/** Governance (decision, vote, quorum) errors. */
export class GovernanceError extends ExochainError {
    name = 'GovernanceError';
}
/** Authority chain construction and validation errors. */
export class AuthorityError extends ExochainError {
    name = 'AuthorityError';
}
/** Constitutional kernel errors (reserved for future kernel integration). */
export class KernelError extends ExochainError {
    name = 'KernelError';
}
/** Cryptographic operation errors (hashing, signing, verification). */
export class CryptoError extends ExochainError {
    name = 'CryptoError';
}
/** HTTP or network transport errors when talking to exo-gateway. */
export class TransportError extends ExochainError {
    name = 'TransportError';
    status;
    body;
    constructor(message, options) {
        super(message, options);
        if (options?.status !== undefined) {
            this.status = options.status;
        }
        if (options?.body !== undefined) {
            this.body = options.body;
        }
    }
}
//# sourceMappingURL=errors.js.map