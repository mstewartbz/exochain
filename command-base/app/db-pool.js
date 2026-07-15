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
