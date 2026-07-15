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
