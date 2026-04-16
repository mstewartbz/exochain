/**
 * Typed error hierarchy for the EXOCHAIN SDK.
 *
 * All SDK errors extend {@link ExochainError}, which in turn extends the
 * standard `Error` class. Each subtype is thrown by the corresponding domain
 * module so callers can discriminate on `instanceof` without string matching.
 */
/** Base class for all SDK errors. */
export declare class ExochainError extends Error {
    readonly name: string;
    constructor(message: string, options?: {
        cause?: unknown;
    });
}
/** Identity / DID validation and keypair errors. */
export declare class IdentityError extends ExochainError {
    readonly name = "IdentityError";
}
/** Consent / bailment builder and proposal errors. */
export declare class ConsentError extends ExochainError {
    readonly name = "ConsentError";
}
/** Governance (decision, vote, quorum) errors. */
export declare class GovernanceError extends ExochainError {
    readonly name = "GovernanceError";
}
/** Authority chain construction and validation errors. */
export declare class AuthorityError extends ExochainError {
    readonly name = "AuthorityError";
}
/** Constitutional kernel errors (reserved for future kernel integration). */
export declare class KernelError extends ExochainError {
    readonly name = "KernelError";
}
/** Cryptographic operation errors (hashing, signing, verification). */
export declare class CryptoError extends ExochainError {
    readonly name = "CryptoError";
}
/** HTTP or network transport errors when talking to exo-gateway. */
export declare class TransportError extends ExochainError {
    readonly name = "TransportError";
    readonly status?: number;
    readonly body?: string;
    constructor(message: string, options?: {
        cause?: unknown;
        status?: number;
        body?: string;
    });
}
//# sourceMappingURL=errors.d.ts.map