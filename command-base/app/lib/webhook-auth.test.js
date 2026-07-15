'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');

const {
  configureWebhookSecretSetting,
  requireWebhookSecret,
} = require('./webhook-auth');

class SettingsDb {
  constructor(initialValue, hasRow = true) {
    this.hasRow = hasRow;
    this.value = initialValue;
    this.operations = [];
  }

  prepare(sql) {
    return {
      get: (key) => {
        this.operations.push(['get', sql, key]);
        if (key !== 'webhook_secret' || !this.hasRow) return undefined;
        return { value: this.value };
      },
      run: (...params) => {
        this.operations.push(['run', sql, ...params]);
        if (/INSERT OR IGNORE INTO system_settings/.test(sql)) {
          const [key, value] = params;
          if (key === 'webhook_secret' && !this.hasRow) {
            this.hasRow = true;
            this.value = value;
          }
          return { changes: this.hasRow ? 0 : 1 };
        }
        if (/UPDATE system_settings/.test(sql)) {
          const [value, key] = params;
          if (key === 'webhook_secret' && this.hasRow) {
            this.value = value;
            return { changes: 1 };
          }
        }
        return { changes: 0 };
      },
    };
  }
}

function requestWithSecret(secret) {
  return { headers: { 'x-webhook-secret': secret } };
}

const CONFIGURED_SECRET = 'commandbase-test-webhook-secret-32-bytes';

test('missing webhook secret setting fails closed before payload handling', () => {
  const result = requireWebhookSecret({ headers: {} }, new SettingsDb(undefined, false));

  assert.equal(result.ok, false);
  assert.equal(result.status, 503);
  assert.deepEqual(result.body, { error: 'Webhook secret is not securely configured' });
});

test('empty webhook secret setting fails closed before payload handling', () => {
  const result = requireWebhookSecret({ headers: {} }, new SettingsDb(''));

  assert.equal(result.ok, false);
  assert.equal(result.status, 503);
  assert.deepEqual(result.body, { error: 'Webhook secret is not securely configured' });
});

test('short webhook secret settings and environment values fail closed', () => {
  const result = requireWebhookSecret(requestWithSecret('short-secret'), new SettingsDb('short-secret'));
  assert.deepEqual(
    result,
    { ok: false, status: 503, body: { error: 'Webhook secret is not securely configured' } },
  );

  assert.throws(
    () => configureWebhookSecretSetting(new SettingsDb('', true), {
      COMMANDBASE_WEBHOOK_SECRET: 'short-secret',
    }),
    /at least 32 bytes/,
  );
});

test('configured webhook secret rejects missing, query, and incorrect credentials', () => {
  const db = new SettingsDb(CONFIGURED_SECRET);

  assert.deepEqual(
    requireWebhookSecret({ headers: {} }, db),
    { ok: false, status: 401, body: { error: 'Unauthorized' } }
  );
  assert.deepEqual(
    requireWebhookSecret({ headers: {}, query: { secret: CONFIGURED_SECRET } }, db),
    { ok: false, status: 401, body: { error: 'Unauthorized' } }
  );
  assert.deepEqual(
    requireWebhookSecret(requestWithSecret('wrong-secret'), db),
    { ok: false, status: 401, body: { error: 'Unauthorized' } }
  );
});

test('configured webhook secret accepts only the matching header value', () => {
  const result = requireWebhookSecret(requestWithSecret(CONFIGURED_SECRET), new SettingsDb(CONFIGURED_SECRET));

  assert.deepEqual(result, { ok: true });
});

test('webhook secret bootstrap uses environment configuration without overwriting a configured secret', () => {
  const emptyDb = new SettingsDb('', true);
  configureWebhookSecretSetting(emptyDb, { COMMANDBASE_WEBHOOK_SECRET: CONFIGURED_SECRET });
  assert.equal(emptyDb.value, CONFIGURED_SECRET);

  const missingDb = new SettingsDb(undefined, false);
  configureWebhookSecretSetting(missingDb, { COMMANDBASE_WEBHOOK_SECRET: CONFIGURED_SECRET });
  assert.equal(missingDb.value, CONFIGURED_SECRET);

  const existingSecret = 'commandbase-existing-webhook-secret-32-bytes';
  const existingDb = new SettingsDb(existingSecret, true);
  configureWebhookSecretSetting(existingDb, { COMMANDBASE_WEBHOOK_SECRET: CONFIGURED_SECRET });
  assert.equal(existingDb.value, existingSecret);
});

test('webhook secret bootstrap permits an unset secret only as a fail-closed placeholder', () => {
  const db = new SettingsDb(undefined, false);

  configureWebhookSecretSetting(db, {});

  assert.equal(db.value, '');
  assert.equal(requireWebhookSecret(requestWithSecret('anything'), db).status, 503);
});
