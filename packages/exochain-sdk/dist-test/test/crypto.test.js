import { test } from 'node:test';
import { strictEqual, ok, throws, deepStrictEqual } from 'node:assert/strict';
import { sha256, sha256Hex, sha256Hash, bytesToHex, hexToBytes, } from '../src/crypto/hash.js';
import { CryptoError } from '../src/errors.js';
test('sha256 of empty input produces the known digest', async () => {
    const bytes = await sha256(new Uint8Array(0));
    strictEqual(bytes.length, 32);
    strictEqual(bytesToHex(bytes), 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
});
test('sha256Hex matches the known "abc" digest', async () => {
    const hex = await sha256Hex(new TextEncoder().encode('abc'));
    strictEqual(hex, 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
});
test('sha256Hash returns a branded Hash256 (64 chars of hex)', async () => {
    const h = await sha256Hash(new TextEncoder().encode('x'));
    strictEqual(h.length, 64);
    ok(/^[0-9a-f]+$/.test(h));
});
test('bytesToHex / hexToBytes are inverses', () => {
    const bytes = new Uint8Array([0, 1, 15, 16, 255]);
    const hex = bytesToHex(bytes);
    strictEqual(hex, '00010f10ff');
    deepStrictEqual(hexToBytes(hex), bytes);
});
test('hexToBytes rejects odd-length input', () => {
    throws(() => hexToBytes('abc'), CryptoError);
});
test('hexToBytes rejects non-hex characters', () => {
    throws(() => hexToBytes('zz'), CryptoError);
});
test('Different inputs produce different digests', async () => {
    const a = await sha256Hex(new TextEncoder().encode('a'));
    const b = await sha256Hex(new TextEncoder().encode('b'));
    ok(a !== b);
});
//# sourceMappingURL=crypto.test.js.map