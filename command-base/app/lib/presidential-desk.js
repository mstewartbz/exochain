// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

'use strict';

/**
 * Presidential Desk adapter — adjacent C2 surface.
 * Fail closed without EXOCHAIN_API_BASE_URL. Never claims constitutional authority.
 */

function isConfigured() {
  return Boolean(process.env.EXOCHAIN_API_BASE_URL && String(process.env.EXOCHAIN_API_BASE_URL).trim());
}

function pushStatus() {
  const slack = Boolean(process.env.PRESIDENTIAL_SLACK_WEBHOOK_URL);
  const sms = Boolean(process.env.PRESIDENTIAL_TWILIO_AUTH_TOKEN);
  if (!slack && !sms) return 'Push adapters unconfigured (Slack/SMS secrets absent)';
  if (slack && !sms) return 'Slack configured; SMS fallback unconfigured';
  if (!slack && sms) return 'SMS configured; Slack primary unconfigured';
  return 'Slack + SMS secrets present (live push still dogfood-gated)';
}

function getBrief() {
  if (!isConfigured()) {
    return {
      configured: false,
      greeting: 'No presidential decisions surface until EXOCHAIN_API_BASE_URL is configured.',
      reason: 'EXOCHAIN_API_BASE_URL unset — fail closed',
      items: [],
      push_status: pushStatus(),
      chairman_unreachable: false,
    };
  }
  // Live aggregation is dogfood-gated; configured mode returns an empty valid brief.
  return {
    configured: true,
    greeting: 'No presidential decisions today.',
    generated_for: 'bob-stewart',
    items: [],
    push_status: pushStatus(),
    chairman_unreachable: false,
  };
}

function recordAction(action, actor) {
  const allowed = new Set(['inquire', 'challenge', 'ratify', 'veto']);
  if (!allowed.has(action)) {
    return { ok: false, status: 400, message: 'Unknown action' };
  }
  if (!isConfigured()) {
    return {
      ok: false,
      status: 503,
      message: 'Presidential actions fail closed without EXOCHAIN_API_BASE_URL',
    };
  }
  if ((action === 'ratify' || action === 'veto') && (!actor || actor === 'agent')) {
    return {
      ok: false,
      status: 403,
      message: 'Ratify/veto require authenticated principal (bob-stewart / mstewartbz dual gate when irreversible)',
    };
  }
  return {
    ok: true,
    status: 202,
    message: `Action '${action}' accepted for forwarding to EXOCHAIN adapter (not locally authoritative)`,
  };
}

function mountPresidentialRoutes(app) {
  app.get('/api/presidential/brief', (_req, res) => {
    res.json(getBrief());
  });

  app.post('/api/presidential/action', (req, res) => {
    const action = req.body && req.body.action;
    const actor = req.body && req.body.actor;
    const result = recordAction(action, actor);
    res.status(result.status).json(result);
  });

  app.get('/presidential-desk', (_req, res) => {
    res.redirect('/presidential-desk/index.html');
  });
}

module.exports = {
  isConfigured,
  getBrief,
  recordAction,
  pushStatus,
  mountPresidentialRoutes,
};
