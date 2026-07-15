'use strict';

const assert = require('node:assert/strict');
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const test = require('node:test');

const workerRoot = __dirname;

function readWorkerFile(...parts) {
  return readFileSync(join(workerRoot, ...parts), 'utf8');
}

function productionDagDbEnv() {
  return {
    NODE_ENV: 'production',
    COMMAND_BASE_DAGDB_GATEWAY_URL: 'http://127.0.0.1:9999',
    COMMAND_BASE_DAGDB_AUTH_TOKEN: 'test-token',
    COMMAND_BASE_DAGDB_TENANT_ID: 'tenant-worker',
    COMMAND_BASE_DAGDB_NAMESPACE: 'commandbase',
    COMMAND_BASE_DAGDB_OWNER_DID: 'did:exo:worker-owner',
    COMMAND_BASE_DAGDB_CONTROLLER_DID: 'did:exo:worker-controller',
    COMMAND_BASE_DAGDB_SUBMITTED_BY_DID: 'did:exo:worker',
    COMMAND_BASE_DAGDB_WRITE_SIGNATURE: 'a'.repeat(128),
  };
}

test('CommandBase worker production source uses DAG DB adapter instead of SQLite', () => {
  const packageJson = JSON.parse(readWorkerFile('package.json'));
  assert.equal(packageJson.scripts && packageJson.scripts.test, 'node --test');
  assert.equal(
    Object.prototype.hasOwnProperty.call(packageJson.dependencies || {}, 'better-sqlite3'),
    false,
    'worker must not ship better-sqlite3 as a production dependency',
  );

  const source = readWorkerFile('index.js');
  for (const forbidden of [
    "require('better-sqlite3')",
    'require("better-sqlite3")',
    'new Database',
    'the_team.db',
  ]) {
    assert.equal(source.includes(forbidden), false, `worker source must not contain ${forbidden}`);
  }
  assert.equal(source.includes('./worker-db'), true);
  assert.equal(source.includes('require.main === module'), true);

  const dbSource = readWorkerFile('worker-db.js');
  assert.equal(dbSource.includes('../app/lib/commandbase-db-factory'), true);
});

test('CommandBase worker production database opens the DAG DB adapter without starting the poll loop', () => {
  const worker = require('./worker-db');
  const env = productionDagDbEnv();
  const db = worker.openWorkerDatabase(env);

  assert.equal(worker.WORKER_DATABASE_ID, 'commandbase-worker');
  assert.equal(worker.databaseLabel(env), 'DAG DB adapter commandbase-worker');
  assert.equal(db.constructor.name, 'CommandBaseDagDbAdapter');
  assert.equal(db.databaseId, 'commandbase-worker');
  db.close();
});

test('CommandBase worker database boundary fails closed without production DAG DB config', () => {
  const worker = require('./worker-db');

  assert.throws(
    () => worker.openWorkerDatabase({ NODE_ENV: 'production' }),
    /missing required config/,
  );
  assert.equal(
    worker.databaseLabel({
      NODE_ENV: 'development',
      COMMAND_BASE_WORKER_DEV_SQLITE: '/tmp/commandbase-worker-dev.sqlite',
    }),
    '/tmp/commandbase-worker-dev.sqlite',
  );
});

test('CommandBase worker entrypoint can be imported without starting the loop', () => {
  const worker = require('./index');

  assert.equal(worker.WORKER_DATABASE_ID, 'commandbase-worker');
  assert.equal(typeof worker.startWorker, 'function');
  assert.equal(typeof worker.shutdownWorker, 'function');
});
