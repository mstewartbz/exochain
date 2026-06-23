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

const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const test = require('node:test');

const APP_ROOT = __dirname;

function readAppFile(...parts) {
  return readFileSync(join(APP_ROOT, ...parts), 'utf8');
}

const PRODUCTION_DB_FILES = [
  'server.js',
  'db-pool.js',
  'lib/db.js',
  'lib/task-force-db.js',
];

const DURABLE_DASHBOARD_KEYS = [
  'dashboard_widgets',
  'dashboard_widgets_v2',
  'dashboard_widgets_v3',
  'dashboard_widgets_v4',
  'dashboard_widgets_v5',
  'dashboard_locked',
  'dashboard_presets',
  'dashboard_active_preset',
  'dashboard_grid_layout',
  'dashboard_grid_updated',
];

const TEST_GATEWAY_SCRIPT = `
const http = require('node:http');
const noResult = process.env.COMMANDBASE_TEST_NO_RESULT === '1';
const server = http.createServer((req, res) => {
  const chunks = [];
  req.on('data', (chunk) => chunks.push(chunk));
  req.on('end', () => {
    const body = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    const kind = String(body.requested_action || '').replace('commandbase:', '');
    const response = {
      schema_version: 'dagdb_intake_response_v1',
      tenant_id: body.tenant_id,
      namespace: body.namespace,
      idempotency_key: body.idempotency_key,
      memory_id: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      receipt_hash: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      validation_status: 'pending',
      council_status: 'not_required',
      dag_finality_status: 'proposed',
      risk_class: 'low',
      risk_bp: 0,
      created_new: true,
      title: { value: 'title', redacted: false, decision: 'allow' },
      summary: { value: 'summary', redacted: false, decision: 'allow' },
      keywords: [],
      validation_report_id: null,
      council_decision_id: null,
      duplicate_of_memory_id: null
    };
    if (!noResult) {
      if (kind === 'run') response.commandbase_result = { changes: 1, lastInsertRowid: 42 };
      if (kind === 'get') response.commandbase_result = { row: { action: body.requested_action, scope: req.headers['x-exo-authority-scope'] } };
      if (kind === 'all' || kind === 'pragma') response.commandbase_result = { rows: [{ action: body.requested_action, tenant: req.headers['x-exo-tenant-id'] }] };
    }
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(JSON.stringify(response));
  });
});
server.listen(0, '127.0.0.1', () => {
  process.stdout.write(JSON.stringify({ port: server.address().port }) + '\\n');
});
process.on('SIGTERM', () => server.close(() => process.exit(0)));
`;

function startGateway(extraEnv) {
  const child = spawn(process.execPath, ['-e', TEST_GATEWAY_SCRIPT], {
    env: { ...process.env, ...(extraEnv || {}) },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return new Promise((resolve, reject) => {
    let stdout = '';
    let stderr = '';
    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`test gateway did not start: ${stderr}`));
    }, 5000);
    child.stderr.on('data', (chunk) => { stderr += chunk.toString(); });
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
      const newline = stdout.indexOf('\n');
      if (newline !== -1) {
        clearTimeout(timer);
        try {
          const parsed = JSON.parse(stdout.slice(0, newline));
          resolve({ child, url: `http://127.0.0.1:${parsed.port}` });
        } catch (error) {
          child.kill('SIGTERM');
          reject(error);
        }
      }
    });
    child.on('error', (error) => {
      clearTimeout(timer);
      reject(error);
    });
  });
}

function adapterEnv(url) {
  return {
    COMMAND_BASE_DAGDB_GATEWAY_URL: url,
    COMMAND_BASE_DAGDB_AUTH_TOKEN: 'test-token',
    COMMAND_BASE_DAGDB_TENANT_ID: 'tenant-a',
    COMMAND_BASE_DAGDB_NAMESPACE: 'primary',
    COMMAND_BASE_DAGDB_OWNER_DID: 'did:exo:owner',
    COMMAND_BASE_DAGDB_CONTROLLER_DID: 'did:exo:controller',
    COMMAND_BASE_DAGDB_SUBMITTED_BY_DID: 'did:exo:submitter',
    COMMAND_BASE_DAGDB_WRITE_SIGNATURE: 'a'.repeat(128),
  };
}

test('CommandBase production database entrypoints use the DAG DB adapter contract', () => {
  const packageJson = JSON.parse(readAppFile('package.json'));
  assert.equal(
    Object.prototype.hasOwnProperty.call(packageJson.dependencies || {}, 'better-sqlite3'),
    false,
    'better-sqlite3 must not be a production dependency for CommandBase',
  );
  assert.equal(
    packageJson.scripts && packageJson.scripts.test,
    'node --test',
    'CommandBase must keep a real npm test contract for DAG DB migration guards',
  );

  for (const file of PRODUCTION_DB_FILES) {
    const source = readAppFile(file);
    assert.equal(
      /require\(['"]better-sqlite3['"]\)/.test(source),
      false,
      `${file} must not import better-sqlite3 in a production entrypoint`,
    );
    assert.equal(
      /\bnew\s+Database\s*\(/.test(source),
      false,
      `${file} must not directly open SQLite in a production entrypoint`,
    );
    assert.equal(
      source.includes('the_team.db') || source.includes('task_forces.db'),
      false,
      `${file} must not hard-code legacy SQLite filenames`,
    );
    assert.equal(
      source.includes('commandbase-db-factory'),
      true,
      `${file} must route persistence through lib/commandbase-db-factory`,
    );
  }

  const factory = require('./lib/commandbase-db-factory');
  assert.equal(typeof factory.createCommandBaseDb, 'function');
  assert.equal(typeof factory.createCommandBaseReadPool, 'function');
  assert.equal(typeof factory.createTaskForceDb, 'function');
  assert.equal(typeof factory.requireDagDbConfig, 'function');

  const adapter = require('./lib/commandbase-dagdb-adapter');
  assert.equal(typeof adapter.CommandBaseDagDbAdapter, 'function');
  assert.equal(typeof adapter.createCommandBaseDagDbAdapter, 'function');
});

test('CommandBase durable dashboard state uses the server DAG DB adapter', () => {
  const indexSource = readAppFile('public/index.html');
  const durableScriptIndex = indexSource.indexOf('src="dagdb-durable-state.js"');
  const appScriptIndex = indexSource.indexOf('src="app.js"');
  assert.equal(
    durableScriptIndex !== -1 && appScriptIndex !== -1 && durableScriptIndex < appScriptIndex,
    true,
    'durable-state adapter script must load before app.js',
  );

  const durableAdapter = readAppFile('public/dagdb-durable-state.js');
  assert.equal(durableAdapter.includes('/api/dagdb/commandbase/ui-state'), true);
  assert.equal(durableAdapter.includes('commandBaseDurableState'), true);

  const appSource = readAppFile('public/app.js');
  for (const key of DURABLE_DASHBOARD_KEYS) {
    assert.equal(
      appSource.includes(`localStorage.getItem('${key}')`) || appSource.includes(`localStorage.setItem('${key}'`),
      false,
      `durable dashboard key ${key} must not read/write localStorage directly`,
    );
  }
  assert.equal(appSource.includes('commandBaseDurableState'), true);
});

test('CommandBase DAG DB adapter uses real gateway results and fails closed when they are absent', async (t) => {
  const { createCommandBaseDagDbAdapter } = require('./lib/commandbase-dagdb-adapter');

  const gateway = await startGateway();
  t.after(() => gateway.child.kill('SIGTERM'));
  const db = createCommandBaseDagDbAdapter({
    env: adapterEnv(gateway.url),
    databaseId: 'commandbase-main',
  });

  const run = db.prepare('INSERT INTO tasks (title) VALUES (?)').run('ship');
  assert.deepEqual(run, { changes: 1, lastInsertRowid: 42 });

  const row = db.prepare('SELECT * FROM tasks WHERE id = ?').get(42);
  assert.equal(row.action, 'commandbase:get');
  assert.equal(row.scope, 'dagdb:intake:tenant-a:primary');

  const rows = db.prepare('SELECT * FROM tasks').all();
  assert.deepEqual(rows, [{ action: 'commandbase:all', tenant: 'tenant-a' }]);

  const noResultGateway = await startGateway({ COMMANDBASE_TEST_NO_RESULT: '1' });
  t.after(() => noResultGateway.child.kill('SIGTERM'));
  const noResultDb = createCommandBaseDagDbAdapter({
    env: adapterEnv(noResultGateway.url),
    databaseId: 'commandbase-main',
  });
  assert.throws(
    () => noResultDb.prepare('SELECT * FROM tasks').get(),
    /missing commandbase_result/,
  );
});
