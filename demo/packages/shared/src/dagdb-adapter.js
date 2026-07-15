import crypto from 'node:crypto';

const REQUIRED_ENV = [
  'EXO_DEMO_DAGDB_GATEWAY_URL',
  'EXO_DEMO_DAGDB_AUTH_TOKEN',
  'EXO_DEMO_DAGDB_TENANT_ID',
  'EXO_DEMO_DAGDB_NAMESPACE',
  'EXO_DEMO_DAGDB_OWNER_DID',
  'EXO_DEMO_DAGDB_CONTROLLER_DID',
  'EXO_DEMO_DAGDB_SUBMITTED_BY_DID',
  'EXO_DEMO_DAGDB_WRITE_SIGNATURE',
];

const TEST_STORE_KEY = Symbol.for('exochain.demo.dagdb.test_store');

function sha256Hex(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}

function stableJson(value) {
  if (value === null || value === undefined) return 'null';
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
  if (typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function normalizeGatewayUrl(value) {
  return String(value || '').replace(/\/+$/, '');
}

export function requireDemoDagDbConfig(env = process.env) {
  const missing = REQUIRED_ENV.filter((name) => !String(env[name] || '').trim());
  if (missing.length > 0) {
    throw new Error(`demo DAG DB adapter missing required config: ${missing.join(', ')}`);
  }
  return {
    gatewayUrl: normalizeGatewayUrl(env.EXO_DEMO_DAGDB_GATEWAY_URL),
    authToken: String(env.EXO_DEMO_DAGDB_AUTH_TOKEN),
    tenantId: String(env.EXO_DEMO_DAGDB_TENANT_ID),
    namespace: String(env.EXO_DEMO_DAGDB_NAMESPACE),
    ownerDid: String(env.EXO_DEMO_DAGDB_OWNER_DID),
    controllerDid: String(env.EXO_DEMO_DAGDB_CONTROLLER_DID),
    submittedByDid: String(env.EXO_DEMO_DAGDB_SUBMITTED_BY_DID),
    writeSignature: String(env.EXO_DEMO_DAGDB_WRITE_SIGNATURE),
  };
}

function queryKind(sql) {
  const trimmed = String(sql || '').trim().toLowerCase();
  if (trimmed.startsWith('select')) return 'select';
  if (trimmed.startsWith('with')) return 'select';
  if (trimmed.includes(' returning ')) return 'returning-write';
  if (trimmed.startsWith('insert')) return 'insert';
  if (trimmed.startsWith('update')) return 'update';
  if (trimmed.startsWith('delete')) return 'delete';
  return 'statement';
}

class DemoDagDbStore {
  constructor(serviceName, options = {}) {
    this.serviceName = serviceName;
    this.config = options.config || requireDemoDagDbConfig(options.env || process.env);
  }

  async query(sql, params = []) {
    const kind = queryKind(sql);
    const operation = {
      adapter: 'demo-dagdb-store-v1',
      service_name: this.serviceName,
      kind,
      sql: String(sql || ''),
      params,
    };
    const payload = stableJson(operation);
    const payloadHash = sha256Hex(payload);
    const body = {
      tenant_id: this.config.tenantId,
      namespace: this.config.namespace,
      idempotency_key: `demo:${this.serviceName}:${kind}:${payloadHash.slice(0, 48)}`,
      source_type: 'generated',
      source_hash: sha256Hex(`${this.serviceName}:${kind}:${String(sql || '')}`),
      payload_hash: payloadHash,
      owner_did: this.config.ownerDid,
      controller_did: this.config.controllerDid,
      submitted_by_did: this.config.submittedByDid,
      consent_purpose: kind === 'select' ? 'retrieval' : 'writeback',
      requested_action: `demo:${this.serviceName}:${kind}`,
      title_text: `Demo ${this.serviceName} ${kind}`,
      summary_text: `Demo ${this.serviceName} ${kind} operation ${payloadHash}`,
      payload_uri_hash: null,
      parent_memory_ids: null,
      edge_types: null,
      access_policy_hash: null,
      declared_rights_hash: null,
      keyword_texts: ['demo', this.serviceName, kind],
    };
    const response = await fetch(`${this.config.gatewayUrl}/api/v1/dag-db/intake`, {
      method: 'POST',
      headers: {
        authorization: `Bearer ${this.config.authToken}`,
        'content-type': 'application/json',
        'x-exo-tenant-id': this.config.tenantId,
        'x-exo-namespace': this.config.namespace,
        'x-exo-authority-scope': `dagdb:intake:${this.config.tenantId}:${this.config.namespace}`,
        'x-exo-write-signature': this.config.writeSignature,
      },
      body: JSON.stringify(body),
    });
    const text = await response.text();
    if (!response.ok) {
      throw new Error(`demo DAG DB intake rejected ${this.serviceName} ${kind} query with status ${response.status}: ${text}`);
    }
    let parsed;
    try {
      parsed = text ? JSON.parse(text) : {};
    } catch (error) {
      throw new Error(`demo DAG DB intake returned non-JSON body: ${error.message}`);
    }
    if (!parsed.demo_result || !Array.isArray(parsed.demo_result.rows)) {
      throw new Error(`demo DAG DB ${this.serviceName} ${kind} response missing demo_result.rows; refusing to synthesize query result`);
    }
    return {
      rows: parsed.demo_result.rows,
      rowCount: Number.isInteger(parsed.demo_result.rowCount)
        ? parsed.demo_result.rowCount
        : parsed.demo_result.rows.length,
    };
  }

  async end() {}
}

export function getDemoServiceTestStore() {
  if (!globalThis[TEST_STORE_KEY]) {
    globalThis[TEST_STORE_KEY] = {
      query: async () => ({ rows: [], rowCount: 0 }),
      end: async () => {},
    };
  }
  return globalThis[TEST_STORE_KEY];
}

export function resetDemoServiceTestStore() {
  const store = getDemoServiceTestStore();
  store.query = async () => ({ rows: [], rowCount: 0 });
  store.end = async () => {};
  return store;
}

export function createDemoServiceStore(serviceName, options = {}) {
  if (options.query) {
    return { query: options.query, end: async () => {} };
  }
  if (process.env.VITEST) {
    return getDemoServiceTestStore();
  }
  return new DemoDagDbStore(serviceName, options);
}
