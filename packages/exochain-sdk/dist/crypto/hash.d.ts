/**
 * Hashing primitives backed by the Web Crypto API.
 *
 * The Rust SDK uses BLAKE3 for content-addressing. This pure-JS reference
 * implementation uses SHA-256 instead, since BLAKE3 is not available in
 * Web Crypto. Hashes produced here are NOT interoperable with Rust-produced
 * hashes at the byte level — they serve as client-side content identifiers
 * for proposal IDs, decision IDs, and the like. For canonical fabric
 * hashes (e.g. trust receipts returned from the gateway), trust the
 * server-provided values.
 */
import type { Hash256 } from '../types.js';
/** Compute SHA-256 over `data` and return the raw 32-byte digest. */
export declare function sha256(data: Uint8Array): Promise<Uint8Array>;
/** Compute SHA-256 and return a 64-character lowercase hex string. */
export declare function sha256Hex(data: Uint8Array): Promise<string>;
/** Compute SHA-256 and return a {@link Hash256} branded hex string. */
export declare function sha256Hash(data: Uint8Array): Promise<Hash256>;
/** Encode a byte array as a lowercase hex string. */
export declare function bytesToHex(bytes: Uint8Array): string;
/** Decode a hex string (odd length not permitted) into bytes. */
export declare function hexToBytes(hex: string): Uint8Array;
//# sourceMappingURL=hash.d.ts.map