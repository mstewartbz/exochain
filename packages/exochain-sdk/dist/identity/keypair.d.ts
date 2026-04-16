/**
 * Ed25519 Identity — a DID paired with a keypair.
 *
 * Built on the Web Crypto API, which supports Ed25519 natively on Node 20+
 * and modern browsers. The DID is derived deterministically from the raw
 * public-key bytes as:
 *
 * ```text
 * did:exo: + first 16 hex chars of SHA-256(public_key_bytes)
 * ```
 *
 * The Rust SDK uses BLAKE3 for this derivation; the pure-JS reference
 * implementation uses SHA-256 so the SDK requires no external dependencies.
 * Two identities generated from the same public key will always produce the
 * same DID within this SDK, but that DID will NOT match one produced by the
 * Rust SDK. For applications that need cross-SDK DIDs, obtain the DID from
 * the canonical Rust-side fabric and pass it into {@link Identity.fromKeypair}.
 */
import type { Did } from '../types.js';
/**
 * Derive `did:exo:<first 16 hex chars of SHA-256(publicKey)>`.
 * Exported for advanced callers who need the same derivation without an
 * `Identity` instance.
 */
export declare function deriveDid(publicKey: Uint8Array): Promise<Did>;
/** A DID paired with an Ed25519 keypair and a human-readable label. */
export declare class Identity {
    #private;
    readonly did: Did;
    readonly publicKeyHex: string;
    readonly label: string;
    private constructor();
    /** Generate a fresh identity with a random Ed25519 keypair. */
    static generate(label: string): Promise<Identity>;
    /**
     * Rebuild an identity from an existing raw key pair (hex-encoded 32-byte
     * seed for the private key, and 32-byte raw public key). Useful for tests
     * and deterministic fixtures.
     */
    static fromKeypair(args: {
        label: string;
        publicKeyHex: string;
        privateKeyPkcs8: Uint8Array;
    }): Promise<Identity>;
    /** Sign `message` with this identity's private key. Returns a 64-byte signature. */
    sign(message: Uint8Array): Promise<Uint8Array>;
    /** Verify `signature` over `message` against this identity's public key. */
    verifySelf(message: Uint8Array, signature: Uint8Array): Promise<boolean>;
    /** Verify a signature against an arbitrary raw public-key hex string. */
    static verify(publicKeyHex: string, message: Uint8Array, signature: Uint8Array): Promise<boolean>;
}
//# sourceMappingURL=keypair.d.ts.map