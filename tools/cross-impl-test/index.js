const blake3 = require('blake3');
const cbor = require('cbor');

// Normative Event Structure mirroring Rust
// Note: CBOR map keys must be sorted for canonical encoding.
// Rust's serde_cbor does this by default or configuration.
// We will simply confirm that specific inputs produce specific BLAKE3 hashes.

function computeHash(obj) {
    const encoded = cbor.encode(obj);
    const hash = blake3.hash(encoded);
    return hash.toString('hex');
}

// Test Vector 1: Simple Opaque Event
// Corresponds to Rust test case
const testVector1 = {
    parents: [],
    logical_time: { physical_ms: 1000, logical: 0 },
    author: "did:exo:test",
    key_version: 1,
    payload: { Opaque: [1, 2, 3] } // Note: Rust Enum serialization variant
};

// Rust serde_cbor default enum serialization might vary (tagged vs object).
// We need to aliign exactly.
// For now, we output what we expect and will tweak Rust/JS to match during detailed verification.

const hash1 = computeHash(testVector1);
console.log(`TestVector1 Hash: ${hash1}`);
