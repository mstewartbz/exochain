'use strict';

const Database = require('better-sqlite3');

function applyCommandBasePragmas(conn, options) {
  const readOnly = options && options.readonly;
  conn.pragma('journal_mode = WAL');
  conn.pragma('foreign_keys = ON');
  conn.pragma(`busy_timeout = ${parseInt(process.env.DB_BUSY_TIMEOUT || '5000', 10)}`);
  conn.pragma('synchronous = NORMAL');
  conn.pragma(`cache_size = ${parseInt(process.env.DB_CACHE_SIZE || '-65536', 10)}`);
  conn.pragma('temp_store = MEMORY');
  conn.pragma(`mmap_size = ${parseInt(process.env.DB_MMAP_SIZE || '268435456', 10)}`);
  if (readOnly) conn.pragma('query_only = ON');
}

function openSqliteDevDatabase(dbPath, options) {
  const conn = new Database(dbPath, {
    fileMustExist: Boolean(options && options.fileMustExist),
    readonly: Boolean(options && options.readonly),
  });
  applyCommandBasePragmas(conn, options || {});
  return conn;
}

module.exports = {
  applyCommandBasePragmas,
  openSqliteDevDatabase,
};
