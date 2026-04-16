/**
 * Typed error hierarchy for the EXOCHAIN SDK.
 *
 * All SDK errors extend {@link ExochainError}, which in turn extends the
 * standard `Error` class. Each subtype is thrown by the corresponding domain
 * module so callers can discriminate on `instanceof` without string matching.
 */

/** Base class for all SDK errors. */
export class ExochainError extends Error {
  public override readonly name: string = 'ExochainError';

  constructor(message: string, options?: { cause?: unknown }) {
    super(message, options);
    // Preserve prototype chain across targets that down-compile class inheritance.
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/** Identity / DID validation and keypair errors. */
export class IdentityError extends ExochainError {
  public override readonly name = 'IdentityError';
}

/** Consent / bailment builder and proposal errors. */
export class ConsentError extends ExochainError {
  public override readonly name = 'ConsentError';
}

/** Governance (decision, vote, quorum) errors. */
export class GovernanceError extends ExochainError {
  public override readonly name = 'GovernanceError';
}

/** Authority chain construction and validation errors. */
export class AuthorityError extends ExochainError {
  public override readonly name = 'AuthorityError';
}

/** Constitutional kernel errors (reserved for future kernel integration). */
export class KernelError extends ExochainError {
  public override readonly name = 'KernelError';
}

/** Cryptographic operation errors (hashing, signing, verification). */
export class CryptoError extends ExochainError {
  public override readonly name = 'CryptoError';
}

/** HTTP or network transport errors when talking to exo-gateway. */
export class TransportError extends ExochainError {
  public override readonly name = 'TransportError';
  public readonly status?: number;
  public readonly body?: string;

  constructor(
    message: string,
    options?: { cause?: unknown; status?: number; body?: string },
  ) {
    super(message, options);
    if (options?.status !== undefined) {
      this.status = options.status;
    }
    if (options?.body !== undefined) {
      this.body = options.body;
    }
  }
}
