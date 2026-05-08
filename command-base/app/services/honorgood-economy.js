'use strict';

class ExochainEconomyClient {
  constructor(options = {}) {
    this.baseUrl = (options.baseUrl || process.env.EXOCHAIN_API_BASE_URL || '').replace(/\/+$/, '');
    this.token = options.token || process.env.EXOCHAIN_API_TOKEN || '';
    this.fetch = options.fetch || globalThis.fetch;
  }

  configured() {
    return Boolean(this.baseUrl);
  }

  status() {
    return {
      configured: this.configured(),
      base_url: this.baseUrl || null,
      settlement_authority: 'EXOCHAIN',
      surface_role: 'CommandBase cockpit adapter',
    };
  }

  async request(path, body) {
    if (!this.baseUrl) {
      throw new Error('EXOCHAIN_API_BASE_URL is required for HonorGood economy actions');
    }
    if (typeof this.fetch !== 'function') {
      throw new Error('fetch is required for HonorGood economy actions');
    }
    const headers = {
      accept: 'application/json',
      'content-type': 'application/json',
    };
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const response = await this.fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers,
      body: JSON.stringify(body || {}),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(`EXOCHAIN economy request failed ${response.status}: ${text}`);
    }
    return text ? JSON.parse(text) : null;
  }

  async get(path) {
    if (!this.baseUrl) {
      throw new Error('EXOCHAIN_API_BASE_URL is required for HonorGood economy reads');
    }
    if (typeof this.fetch !== 'function') {
      throw new Error('fetch is required for HonorGood economy reads');
    }
    const headers = { accept: 'application/json' };
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const response = await this.fetch(`${this.baseUrl}${path}`, { method: 'GET', headers });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(`EXOCHAIN economy read failed ${response.status}: ${text}`);
    }
    return text ? JSON.parse(text) : null;
  }

  createMission(body) {
    return this.request('/api/v1/economy/missions', body);
  }

  createContributionReceipt(body) {
    return this.request('/api/v1/economy/contribution-receipts', body);
  }

  createLegacyReceipt(body) {
    return this.request('/api/v1/economy/legacy-receipts', body);
  }

  getLegacyReceipt(id) {
    return this.get(`/api/v1/economy/legacy-receipts/${encodeURIComponent(id)}`);
  }

  createRuleset(body) {
    return this.request('/api/v1/economy/rulesets', body);
  }

  createMissionSettlement(body) {
    return this.request('/api/v1/economy/mission-settlements', body);
  }

  getMissionSettlement(id) {
    return this.get(`/api/v1/economy/mission-settlements/${encodeURIComponent(id)}`);
  }
}

module.exports = {
  ExochainEconomyClient,
};
