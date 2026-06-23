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

const crypto = require('node:crypto');
const { spawnSync } = require('node:child_process');

const REQUIRED_ENV = [
  'COMMAND_BASE_DAGDB_GATEWAY_URL',
  'COMMAND_BASE_DAGDB_AUTH_TOKEN',
  'COMMAND_BASE_DAGDB_TENANT_ID',
  'COMMAND_BASE_DAGDB_NAMESPACE',
  'COMMAND_BASE_DAGDB_OWNER_DID',
  'COMMAND_BASE_DAGDB_CONTROLLER_DID',
  'COMMAND_BASE_DAGDB_SUBMITTED_BY_DID',
  'COMMAND_BASE_DAGDB_WRITE_SIGNATURE',
];

const HTTP_BRIDGE_SCRIPT = `
const chunks = [];
process.stdin.on('data', (chunk) => chunks.push(chunk));
process.stdin.on('end', async () => {
  try {
    const request = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    const response = await fetch(request.url, {
      method: request.method,
      headers: request.headers,
      body: JSON.stringify(request.body)
    });
    const text = await response.text();
    process.stdout.write(JSON.stringify({
      ok: response.ok,
      status: response.status,
      body: text
    }));
  } catch (error) {
    process.stderr.write(error && error.stack ? error.stack : String(error));
    process.exit(1);
  }
});
`;

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

function normalizeGatewayUrl(url) {
  return String(url || '').replace(/\/+$/, '');
}

function requireDagDbConfig(env) {
  const source = env || process.env;
  const missing = REQUIRED_ENV.filter((name) => !String(source[name] || '').trim());
  if (missing.length > 0) {
    throw new Error(`CommandBase DAG DB adapter missing required config: ${missing.join(', ')}`);
  }
  return {
    gatewayUrl: normalizeGatewayUrl(source.COMMAND_BASE_DAGDB_GATEWAY_URL),
    authToken: String(source.COMMAND_BASE_DAGDB_AUTH_TOKEN),
    tenantId: String(source.COMMAND_BASE_DAGDB_TENANT_ID),
    namespace: String(source.COMMAND_BASE_DAGDB_NAMESPACE),
    ownerDid: String(source.COMMAND_BASE_DAGDB_OWNER_DID),
    controllerDid: String(source.COMMAND_BASE_DAGDB_CONTROLLER_DID),
    submittedByDid: String(source.COMMAND_BASE_DAGDB_SUBMITTED_BY_DID),
    writeSignature: String(source.COMMAND_BASE_DAGDB_WRITE_SIGNATURE),
  };
}

class DagDbStatement {
  constructor(adapter, sql) {
    this.adapter = adapter;
    this.sql = sql;
  }

  run(...params) {
    const result = this.adapter.recordSqlOperation('run', this.sql, params);
    return {
      changes: result.changes,
      lastInsertRowid: result.lastInsertRowid,
    };
  }

  get(...params) {
    return this.adapter.recordSqlOperation('get', this.sql, params).row;
  }

  all(...params) {
    return this.adapter.recordSqlOperation('all', this.sql, params).rows;
  }
}

class CommandBaseDagDbAdapter {
  constructor(options) {
    const cfg = options && options.config ? options.config : requireDagDbConfig(options && options.env);
    this.config = cfg;
    this.databaseId = (options && options.databaseId) || 'commandbase-main';
    this.readonly = Boolean(options && options.readonly);
    this.sequence = 0;
  }

  pragma(statement) {
    return this.recordSqlOperation('pragma', String(statement || ''), []).rows;
  }

  exec(sql) {
    this.recordSqlOperation('exec', String(sql || ''), []);
  }

  prepare(sql) {
    return new DagDbStatement(this, String(sql || ''));
  }

  transaction(fn) {
    if (typeof fn !== 'function') {
      throw new Error('CommandBase DAG DB transaction requires a function');
    }
    return (...args) => fn(...args);
  }

  close() {}

  recordDurableState(key, value) {
    const sql = 'commandbase.durable_state.set';
    return this.recordSqlOperation('durable-state', sql, [String(key), String(value)]);
  }

  recordSqlOperation(kind, sql, params) {
    if (this.readonly && ['run', 'exec', 'durable-state'].includes(kind)) {
      throw new Error(`CommandBase DAG DB read adapter refused write operation ${kind}`);
    }
    const operation = {
      adapter: 'commandbase-dagdb-adapter-v1',
      database_id: this.databaseId,
      kind,
      sql,
      params,
    };
    const payload = stableJson(operation);
    const payloadHash = sha256Hex(payload);
    const sourceHash = sha256Hex(`${this.databaseId}:${kind}:${sql}`);
    const idempotencyKey = `commandbase:${this.databaseId}:${kind}:${payloadHash.slice(0, 48)}`;
    const response = this.sendIntake({
      tenant_id: this.config.tenantId,
      namespace: this.config.namespace,
      idempotency_key: idempotencyKey,
      source_type: 'generated',
      source_hash: sourceHash,
      payload_hash: payloadHash,
      owner_did: this.config.ownerDid,
      controller_did: this.config.controllerDid,
      submitted_by_did: this.config.submittedByDid,
      consent_purpose: kind === 'get' || kind === 'all' ? 'retrieval' : 'writeback',
      requested_action: `commandbase:${kind}`,
      title_text: `CommandBase ${this.databaseId} ${kind}`,
      summary_text: `CommandBase ${this.databaseId} ${kind} operation ${payloadHash}`,
      payload_uri_hash: null,
      parent_memory_ids: null,
      edge_types: null,
      access_policy_hash: null,
      declared_rights_hash: null,
      keyword_texts: ['commandbase', this.databaseId, kind],
    });
    this.sequence += 1;
    return this.statementResult(kind, response.body);
  }

  statementResult(kind, body) {
    if (kind === 'exec' || kind === 'durable-state') {
      return { changes: 0, lastInsertRowid: null, row: undefined, rows: [] };
    }
    const result = body && body.commandbase_result;
    if (kind === 'pragma') {
      if (!result) return { changes: 0, lastInsertRowid: null, row: undefined, rows: [] };
      return {
        changes: 0,
        lastInsertRowid: null,
        row: undefined,
        rows: this.requireRows(result.rows, kind),
      };
    }
    if (!result || typeof result !== 'object') {
      throw new Error(`CommandBase DAG DB ${kind} response missing commandbase_result; refusing to synthesize SQL result`);
    }
    if (kind === 'get') {
      return {
        changes: 0,
        lastInsertRowid: null,
        row: result.row === null ? undefined : result.row,
        rows: [],
      };
    }
    if (kind === 'all') {
      return {
        changes: 0,
        lastInsertRowid: null,
        row: undefined,
        rows: this.requireRows(result.rows, kind),
      };
    }
    if (kind === 'run') {
      if (!Number.isInteger(result.changes) || !Object.prototype.hasOwnProperty.call(result, 'lastInsertRowid')) {
        throw new Error('CommandBase DAG DB run response must include integer changes and lastInsertRowid');
      }
      return {
        changes: result.changes,
        lastInsertRowid: result.lastInsertRowid,
        row: undefined,
        rows: [],
      };
    }
    throw new Error(`CommandBase DAG DB adapter received unsupported operation kind ${kind}`);
  }

  requireRows(rows, kind) {
    if (!Array.isArray(rows)) {
      throw new Error(`CommandBase DAG DB ${kind} response must include commandbase_result.rows array`);
    }
    return rows;
  }

  sendIntake(body) {
    const request = {
      url: `${this.config.gatewayUrl}/api/v1/dag-db/intake`,
      method: 'POST',
      headers: {
        authorization: `Bearer ${this.config.authToken}`,
        'content-type': 'application/json',
        'x-exo-tenant-id': this.config.tenantId,
        'x-exo-namespace': this.config.namespace,
        'x-exo-authority-scope': `dagdb:intake:${this.config.tenantId}:${this.config.namespace}`,
        'x-exo-write-signature': this.config.writeSignature,
      },
      body,
    };
    const child = spawnSync(process.execPath, ['-e', HTTP_BRIDGE_SCRIPT], {
      input: JSON.stringify(request),
      encoding: 'utf8',
      maxBuffer: 1024 * 1024,
    });
    if (child.error) throw child.error;
    if (child.status !== 0) {
      throw new Error(`CommandBase DAG DB intake transport failed: ${String(child.stderr || '').trim()}`);
    }
    let response;
    try {
      response = JSON.parse(child.stdout);
    } catch (error) {
      throw new Error(`CommandBase DAG DB intake returned malformed response: ${error.message}`);
    }
    if (!response.ok) {
      throw new Error(`CommandBase DAG DB intake rejected operation with status ${response.status}: ${response.body}`);
    }
    let parsedBody;
    try {
      parsedBody = response.body ? JSON.parse(response.body) : {};
    } catch (error) {
      throw new Error(`CommandBase DAG DB intake returned non-JSON body: ${error.message}`);
    }
    return {
      status: response.status,
      body: parsedBody,
    };
  }
}

function createCommandBaseDagDbAdapter(options) {
  return new CommandBaseDagDbAdapter(options || {});
}

module.exports = {
  CommandBaseDagDbAdapter,
  createCommandBaseDagDbAdapter,
  requireDagDbConfig,
};
