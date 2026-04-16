import { test } from 'node:test';
import { strictEqual, ok, rejects, throws } from 'node:assert/strict';
import { Identity } from '../src/identity/keypair.js';
import { validateDid, isDid } from '../src/identity/did.js';
import { IdentityError } from '../src/errors.js';

test('Identity.generate produces a well-formed did:exo: DID', async () => {
  const id = await Identity.generate('alice');
  ok(id.did.startsWith('did:exo:'));
  // 16 hex chars of SHA-256 prefix.
  strictEqual(id.did.length, 'did:exo:'.length + 16);
});

test('Identity.generate exposes a 64-char (32-byte) public key hex', async () => {
  const id = await Identity.generate('bob');
  strictEqual(id.publicKeyHex.length, 64);
  ok(/^[0-9a-f]+$/.test(id.publicKeyHex));
});

test('Identity.generate stores the label', async () => {
  const id = await Identity.generate('carol');
  strictEqual(id.label, 'carol');
});

test('Identity sign/verify round-trip succeeds', async () => {
  const id = await Identity.generate('signer');
  const msg = new TextEncoder().encode('hello exochain');
  const sig = await id.sign(msg);
  strictEqual(sig.length, 64);
  ok(await Identity.verify(id.publicKeyHex, msg, sig));
  ok(await id.verifySelf(msg, sig));
});

test('Identity.verify rejects a tampered message', async () => {
  const id = await Identity.generate('signer');
  const sig = await id.sign(new TextEncoder().encode('original'));
  const bad = await Identity.verify(
    id.publicKeyHex,
    new TextEncoder().encode('tampered'),
    sig,
  );
  strictEqual(bad, false);
});

test('Different identities produce different DIDs', async () => {
  const a = await Identity.generate('a');
  const b = await Identity.generate('b');
  ok(a.did !== b.did);
});

test('validateDid accepts well-formed DIDs', () => {
  const d = validateDid('did:exo:abc123');
  strictEqual(d, 'did:exo:abc123');
});

test('validateDid rejects bad input', () => {
  throws(() => validateDid('not-a-did'), IdentityError);
  throws(() => validateDid('did:exo:'), IdentityError);
  throws(() => validateDid('did:other:abc'), IdentityError);
  throws(() => validateDid('did:exo:bad chars!'), IdentityError);
});

test('isDid type-guard returns boolean', () => {
  ok(isDid('did:exo:alice'));
  ok(!isDid('nope'));
});

test('Identity.verify returns false for bad public key input', async () => {
  const result = await Identity.verify('zz', new Uint8Array([1]), new Uint8Array([2]));
  strictEqual(result, false);
});

test('Identity.generate rejects non-string label', async () => {
  await rejects(
    async () => Identity.generate(42 as unknown as string),
    IdentityError,
  );
});
