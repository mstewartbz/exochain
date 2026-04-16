import { test } from 'node:test';
import { strictEqual, ok, throws } from 'node:assert/strict';
import { AuthorityChainBuilder } from '../src/authority/chain.js';
import { AuthorityError, IdentityError } from '../src/errors.js';

const ROOT = 'did:exo:root';
const MID = 'did:exo:mid';
const LEAF = 'did:exo:leaf';

test('Two-link chain validates and terminates at leaf', () => {
  const chain = new AuthorityChainBuilder()
    .addLink(ROOT, MID, ['read'])
    .addLink(MID, LEAF, ['read'])
    .build(LEAF);
  strictEqual(chain.depth, 2);
  strictEqual(chain.terminal, LEAF);
  strictEqual(chain.links[0]?.grantor, ROOT);
  strictEqual(chain.links[1]?.grantee, LEAF);
});

test('Single-link chain is valid', () => {
  const chain = new AuthorityChainBuilder().addLink(ROOT, LEAF, ['all']).build(LEAF);
  strictEqual(chain.depth, 1);
});

test('Empty chain fails', () => {
  throws(() => new AuthorityChainBuilder().build(LEAF), AuthorityError);
});

test('Broken chain fails', () => {
  throws(
    () =>
      new AuthorityChainBuilder()
        .addLink(ROOT, MID, ['read'])
        .addLink('did:exo:other', LEAF, ['read'])
        .build(LEAF),
    AuthorityError,
  );
});

test('Wrong terminal fails', () => {
  throws(
    () =>
      new AuthorityChainBuilder()
        .addLink(ROOT, MID, ['read'])
        .addLink(MID, LEAF, ['read'])
        .build('did:exo:different'),
    AuthorityError,
  );
});

test('Invalid DID passed to addLink is rejected eagerly', () => {
  throws(
    () => new AuthorityChainBuilder().addLink('nope', LEAF, ['read']),
    IdentityError,
  );
});

test('Permissions are copied, not aliased', () => {
  const perms = ['read', 'write'];
  const chain = new AuthorityChainBuilder().addLink(ROOT, LEAF, perms).build(LEAF);
  perms.push('admin');
  strictEqual(chain.links[0]?.permissions.length, 2);
  ok(chain.links[0]?.permissions.includes('read'));
});
