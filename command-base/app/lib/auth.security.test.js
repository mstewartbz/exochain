'use strict';

const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const test = require('node:test');

const authSource = readFileSync(join(__dirname, 'auth.js'), 'utf8');

test('auth module must not include embedded fallback secret', () => {
  assert.equal(
    authSource.includes('exochain-dev-secret-change-in-production'),
    false,
    'embedded fallback secret must be removed from auth.js',
  );
  assert.equal(
    /function getHmacSecret\(\)/.test(authSource),
    true,
    'auth.js must resolve HMAC secret via runtime function',
  );
});
