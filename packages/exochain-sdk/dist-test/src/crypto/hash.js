/**
 * Hashing primitives.
 *
 * BLAKE3 is used where the SDK must match Rust fabric derivations, including
 * local DID derivation. SHA-256 remains available for client-side proposal
 * IDs, decision IDs, and compatibility with existing TypeScript SDK records.
 */
import { blake3 as nobleBlake3 } from '@noble/hashes/blake3';
import { CryptoError } from '../errors.js';
const subtle = (() => {
    const c = globalThis.crypto;
    if (c === undefined || c.subtle === undefined) {
        throw new CryptoError('Web Crypto API is unavailable. Requires Node >= 20 or a modern browser.');
    }
    return c.subtle;
})();
/** Compute SHA-256 over `data` and return the raw 32-byte digest. */
export async function sha256(data) {
    try {
        // The `as BufferSource` cast is required because older TS lib defs typed
        // `digest` as accepting only `ArrayBuffer | ArrayBufferView`, which a
        // `Uint8Array` satisfies at runtime.
        const buf = await subtle.digest('SHA-256', data);
        return new Uint8Array(buf);
    }
    catch (err) {
        throw new CryptoError('SHA-256 digest failed', { cause: err });
    }
}
/** Compute SHA-256 and return a 64-character lowercase hex string. */
export async function sha256Hex(data) {
    const bytes = await sha256(data);
    return bytesToHex(bytes);
}
/** Compute SHA-256 and return a {@link Hash256} branded hex string. */
export async function sha256Hash(data) {
    const hex = await sha256Hex(data);
    return hex;
}
/** Compute BLAKE3 over `data` and return the raw 32-byte digest. */
export function blake3(data) {
    try {
        return nobleBlake3(data);
    }
    catch (err) {
        throw new CryptoError('BLAKE3 digest failed', { cause: err });
    }
}
/** Compute BLAKE3 and return a 64-character lowercase hex string. */
export function blake3Hex(data) {
    return bytesToHex(blake3(data));
}
/** Compute BLAKE3 and return a {@link Hash256} branded hex string. */
export function blake3Hash(data) {
    return blake3Hex(data);
}
/** Encode a byte array as a lowercase hex string. */
export function bytesToHex(bytes) {
    let out = '';
    for (let i = 0; i < bytes.length; i++) {
        const b = bytes[i] ?? 0;
        out += b.toString(16).padStart(2, '0');
    }
    return out;
}
/** Decode a hex string (odd length not permitted) into bytes. */
export function hexToBytes(hex) {
    if (hex.length % 2 !== 0) {
        throw new CryptoError(`hex string has odd length: ${hex.length}`);
    }
    const out = new Uint8Array(hex.length / 2);
    for (let i = 0; i < out.length; i++) {
        const byte = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
        if (Number.isNaN(byte)) {
            throw new CryptoError(`invalid hex at offset ${i * 2}`);
        }
        out[i] = byte;
    }
    return out;
}
//# sourceMappingURL=hash.js.map