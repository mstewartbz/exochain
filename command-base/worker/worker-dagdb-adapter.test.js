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
  assert.equal(source.includes('../app/lib/commandbase-db-factory'), true);
  assert.equal(source.includes('require.main === module'), true);
});

test('CommandBase worker production database opens the DAG DB adapter without starting the poll loop', () => {
  const worker = require('./index');
  const env = productionDagDbEnv();
  const db = worker.openWorkerDatabase(env);

  assert.equal(worker.WORKER_DATABASE_ID, 'commandbase-worker');
  assert.equal(worker.databaseLabel(env), 'DAG DB adapter commandbase-worker');
  assert.equal(db.constructor.name, 'CommandBaseDagDbAdapter');
  assert.equal(db.databaseId, 'commandbase-worker');
  db.close();
});
