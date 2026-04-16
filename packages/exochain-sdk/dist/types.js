/**
 * Core type definitions for the EXOCHAIN SDK.
 *
 * Branded primitive aliases (`Did`, `Hash256`) make it impossible to confuse a
 * validated DID or hash with an arbitrary `string`. They are structural only
 * — there is no runtime tag, so validation happens at the boundary via the
 * factory functions in {@link ./identity/did.ts} and {@link ./crypto/hash.ts}.
 */
export {};
//# sourceMappingURL=types.js.map