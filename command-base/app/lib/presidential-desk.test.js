'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const {
  isConfigured,
  getBrief,
  recordAction,
  pushStatus,
} = require('./presidential-desk.js');

test('presidential desk fails closed without EXOCHAIN_API_BASE_URL', () => {
  const prev = process.env.EXOCHAIN_API_BASE_URL;
  delete process.env.EXOCHAIN_API_BASE_URL;
  assert.equal(isConfigured(), false);
  const brief = getBrief();
  assert.equal(brief.configured, false);
  assert.equal(brief.items.length, 0);
  assert.match(brief.reason, /fail closed/i);
  if (prev === undefined) delete process.env.EXOCHAIN_API_BASE_URL;
  else process.env.EXOCHAIN_API_BASE_URL = prev;
});

test('configured brief returns empty valid attention set', () => {
  const prev = process.env.EXOCHAIN_API_BASE_URL;
  process.env.EXOCHAIN_API_BASE_URL = 'https://exochain.example';
  const brief = getBrief();
  assert.equal(brief.configured, true);
  assert.equal(brief.generated_for, 'bob-stewart');
  assert.deepEqual(brief.items, []);
  if (prev === undefined) delete process.env.EXOCHAIN_API_BASE_URL;
  else process.env.EXOCHAIN_API_BASE_URL = prev;
});

test('ratify rejects agent actor', () => {
  const prev = process.env.EXOCHAIN_API_BASE_URL;
  process.env.EXOCHAIN_API_BASE_URL = 'https://exochain.example';
  const denied = recordAction('ratify', 'agent');
  assert.equal(denied.ok, false);
  assert.equal(denied.status, 403);
  const ok = recordAction('inquire', 'bob-stewart');
  assert.equal(ok.ok, true);
  if (prev === undefined) delete process.env.EXOCHAIN_API_BASE_URL;
  else process.env.EXOCHAIN_API_BASE_URL = prev;
});

test('push status never exposes secret values', () => {
  const prevSlack = process.env.PRESIDENTIAL_SLACK_WEBHOOK_URL;
  process.env.PRESIDENTIAL_SLACK_WEBHOOK_URL = 'https://hooks.slack.com/secret-token-value';
  const status = pushStatus();
  assert.doesNotMatch(status, /secret-token-value/);
  if (prevSlack === undefined) delete process.env.PRESIDENTIAL_SLACK_WEBHOOK_URL;
  else process.env.PRESIDENTIAL_SLACK_WEBHOOK_URL = prevSlack;
});
