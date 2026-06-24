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
 * Read pool for the active CommandBase persistence adapter.
 *
 * Production opens DAG DB-backed read adapters. Test and development retain the
 * compatibility SQLite adapter behind `lib/sqlite-dev-db.js` only.
 */

const {
  createCommandBaseReadPool,
  defaultCommandBaseDbPath,
} = require('./lib/commandbase-db-factory');

const DB_PATH = process.env.DB_PATH || defaultCommandBaseDbPath(__dirname);
const READ_POOL_SIZE = parseInt(process.env.DB_READ_POOL_SIZE || '4', 10);

const pool = createCommandBaseReadPool({
  dbPath: DB_PATH,
  databaseId: 'commandbase-main',
  fileMustExist: true,
  poolSize: READ_POOL_SIZE,
});

process.once('exit', () => pool.close());
process.once('SIGINT', () => { pool.close(); process.exit(0); });
process.once('SIGTERM', () => { pool.close(); process.exit(0); });

module.exports = pool;
