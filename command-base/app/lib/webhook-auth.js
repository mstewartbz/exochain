'use strict';

const crypto = require('node:crypto');

const WEBHOOK_SECRET_KEY = 'webhook_secret';
const MIN_WEBHOOK_SECRET_BYTES = 32;

function normalizeConfiguredSecret(value) {
  return typeof value === 'string' ? value.trim() : '';
}

function hasMinimumSecretBytes(value) {
  return Buffer.byteLength(value, 'utf8') >= MIN_WEBHOOK_SECRET_BYTES;
}

function loadWebhookSecret(db) {
  const row = db
    .prepare('SELECT value FROM system_settings WHERE key = ?')
    .get(WEBHOOK_SECRET_KEY);
  return normalizeConfiguredSecret(row && row.value);
}

function timingSafeSecretEqual(provided, expected) {
  if (typeof provided !== 'string' || typeof expected !== 'string') return false;

  const providedBuffer = Buffer.from(provided, 'utf8');
  const expectedBuffer = Buffer.from(expected, 'utf8');
  if (providedBuffer.length !== expectedBuffer.length) return false;

  return crypto.timingSafeEqual(providedBuffer, expectedBuffer);
}

function readWebhookSecretHeader(req) {
  if (req && typeof req.get === 'function') {
    const header = req.get('x-webhook-secret');
    return typeof header === 'string' ? header : undefined;
  }

  const headers = (req && req.headers) || {};
  const header = headers['x-webhook-secret'] || headers['X-Webhook-Secret'];
  return typeof header === 'string' ? header : undefined;
}

function configureWebhookSecretSetting(db, env = process.env) {
  const configuredSecret = normalizeConfiguredSecret(env.COMMANDBASE_WEBHOOK_SECRET);
  if (configuredSecret && !hasMinimumSecretBytes(configuredSecret)) {
    throw new Error(
      `COMMANDBASE_WEBHOOK_SECRET must be at least ${MIN_WEBHOOK_SECRET_BYTES} bytes`,
    );
  }

  db.prepare('INSERT OR IGNORE INTO system_settings (key, value) VALUES (?, ?)')
    .run(WEBHOOK_SECRET_KEY, configuredSecret);

  if (!configuredSecret) return;

  const storedSecret = loadWebhookSecret(db);
  if (!storedSecret) {
    db.prepare('UPDATE system_settings SET value = ? WHERE key = ?')
      .run(configuredSecret, WEBHOOK_SECRET_KEY);
  }
}

function requireWebhookSecret(req, db) {
  const expectedSecret = loadWebhookSecret(db);
  if (!expectedSecret || !hasMinimumSecretBytes(expectedSecret)) {
    return {
      ok: false,
      status: 503,
      body: { error: 'Webhook secret is not securely configured' },
    };
  }

  const providedSecret = readWebhookSecretHeader(req);
  if (!timingSafeSecretEqual(providedSecret, expectedSecret)) {
    return {
      ok: false,
      status: 401,
      body: { error: 'Unauthorized' },
    };
  }

  return { ok: true };
}

module.exports = {
  MIN_WEBHOOK_SECRET_BYTES,
  configureWebhookSecretSetting,
  requireWebhookSecret,
};
