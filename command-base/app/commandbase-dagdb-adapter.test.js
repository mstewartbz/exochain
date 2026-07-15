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

test('CommandBase DAG DB adapter covers fail-closed local result branches', () => {
  const {
    CommandBaseDagDbAdapter,
    requireDagDbConfig,
  } = require('./lib/commandbase-dagdb-adapter');

  assert.throws(
    () => requireDagDbConfig({ COMMAND_BASE_DAGDB_GATEWAY_URL: 'http://127.0.0.1:1' }),
    /missing required config/,
  );

  const db = new CommandBaseDagDbAdapter({
    config: {
      gatewayUrl: 'http://127.0.0.1:1/',
      authToken: 'token',
      tenantId: 'tenant-a',
      namespace: 'primary',
      ownerDid: 'did:exo:owner',
      controllerDid: 'did:exo:controller',
      submittedByDid: 'did:exo:submitter',
      writeSignature: 'a'.repeat(128),
    },
    databaseId: 'branch-coverage',
  });

  assert.equal(db.config.gatewayUrl, 'http://127.0.0.1:1/');
  assert.throws(() => db.transaction('not-a-function'), /requires a function/);
  assert.equal(db.transaction((value) => `ok:${value}`)('value'), 'ok:value');
  assert.deepEqual(db.statementResult('exec', {}), {
    changes: 0,
    lastInsertRowid: null,
    row: undefined,
    rows: [],
  });
  assert.deepEqual(db.statementResult('durable-state', {}), {
    changes: 0,
    lastInsertRowid: null,
    row: undefined,
    rows: [],
  });
  assert.deepEqual(db.statementResult('pragma', {}), {
    changes: 0,
    lastInsertRowid: null,
    row: undefined,
    rows: [],
  });
  assert.deepEqual(db.statementResult('get', { commandbase_result: { row: null } }).row, undefined);
  assert.deepEqual(db.statementResult('all', { commandbase_result: { rows: [{ id: 1 }] } }).rows, [{ id: 1 }]);
  assert.throws(
    () => db.statementResult('run', { commandbase_result: { changes: '1' } }),
    /must include integer changes/,
  );
  assert.throws(
    () => db.statementResult('all', { commandbase_result: { rows: 'not-array' } }),
    /rows array/,
  );
  assert.throws(
    () => db.statementResult('unknown', { commandbase_result: {} }),
    /unsupported operation kind/,
  );

  const readonly = new CommandBaseDagDbAdapter({
    config: db.config,
    readonly: true,
  });
  assert.throws(
    () => readonly.recordSqlOperation('run', 'INSERT INTO tasks DEFAULT VALUES', []),
    /read adapter refused write operation/,
  );
  assert.throws(
    () => db.prepare('SELECT * FROM tasks').get(1),
    /transport failed/,
  );
});

test('CommandBase DB factory routes production callers to DAG DB and covers read-pool branches', () => {
  const factory = require('./lib/commandbase-db-factory');
  const env = {
    ...adapterEnv('http://127.0.0.1:9999'),
    NODE_ENV: 'production',
  };

  assert.equal(factory.productionUsesDagDb({ NODE_ENV: 'production' }), true);
  assert.equal(
    factory.productionUsesDagDb({ NODE_ENV: 'production', COMMAND_BASE_ALLOW_DEV_SQLITE: '1' }),
    false,
  );
  assert.equal(factory.productionUsesDagDb({ NODE_ENV: 'development' }), false);
  assert.match(factory.defaultCommandBaseDbPath('/tmp/app'), /commandbase-dev\.sqlite$/);
  assert.match(factory.defaultTaskForceDbPath('/tmp/app'), /commandbase-task-forces-dev\.sqlite$/);

  const db = factory.createCommandBaseDb({
    env,
    databaseId: 'factory-main',
    readonly: true,
  });
  assert.equal(db.constructor.name, 'CommandBaseDagDbAdapter');
  assert.equal(db.databaseId, 'factory-main');
  assert.equal(db.readonly, true);

  const taskForceDb = factory.createTaskForceDb({ env });
  assert.equal(taskForceDb.databaseId, 'commandbase-task-forces');

  const pool = factory.createCommandBaseReadPool({
    env,
    poolSize: 2,
    databaseId: 'factory-pool',
  });
  assert.deepEqual(pool.stats(), { size: 2, busy: 0, idle: 2 });
  const first = pool.acquire();
  const second = pool.acquire();
  assert.equal(pool.acquire(), null);
  assert.equal(pool.read((fallback) => fallback.marker, { marker: 'fallback-used' }), 'fallback-used');
  assert.throws(() => pool.read(() => 'unreachable'), /Read pool exhausted/);
  pool.release(first);
  assert.equal(pool.read((conn) => conn.databaseId), 'factory-pool');
  pool.release(second);
  assert.deepEqual(pool.stats(), { size: 2, busy: 0, idle: 2 });
  pool.close();
  assert.deepEqual(pool.stats(), { size: 0, busy: 0, idle: 0 });
});
