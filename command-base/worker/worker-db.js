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

const path = require('path');
const {
  createCommandBaseDb,
  productionUsesDagDb,
} = require('../app/lib/commandbase-db-factory');

const WORKER_DATABASE_ID = 'commandbase-worker';
const DEFAULT_DEV_SQLITE_FILE = path.join(__dirname, '..', 'commandbase-worker-dev.sqlite');

function openWorkerDatabase(env = process.env) {
  return createCommandBaseDb({
    env,
    databaseId: WORKER_DATABASE_ID,
    dbPath: env.COMMAND_BASE_WORKER_DEV_SQLITE || DEFAULT_DEV_SQLITE_FILE,
    fileMustExist: false,
  });
}

function databaseLabel(env = process.env) {
  return productionUsesDagDb(env)
    ? 'DAG DB adapter commandbase-worker'
    : env.COMMAND_BASE_WORKER_DEV_SQLITE || DEFAULT_DEV_SQLITE_FILE;
}

module.exports = {
  databaseLabel,
  openWorkerDatabase,
  WORKER_DATABASE_ID,
};
