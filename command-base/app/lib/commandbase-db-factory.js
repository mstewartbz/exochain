'use strict';

const path = require('node:path');
const {
  createCommandBaseDagDbAdapter,
  requireDagDbConfig,
} = require('./commandbase-dagdb-adapter');

function openSqliteDevDatabase(dbPath, options) {
  return require('./sqlite-dev-db').openSqliteDevDatabase(dbPath, options);
}

function productionUsesDagDb(env) {
  const source = env || process.env;
  return source.NODE_ENV === 'production' && source.COMMAND_BASE_ALLOW_DEV_SQLITE !== '1';
}

function defaultCommandBaseDbPath(rootDir) {
  return path.join(rootDir, '..', 'commandbase-dev.sqlite');
}

function defaultTaskForceDbPath(rootDir) {
  return path.join(rootDir, '..', 'commandbase-task-forces-dev.sqlite');
}

function createCommandBaseDb(options) {
  const opts = options || {};
  if (productionUsesDagDb(opts.env)) {
    return createCommandBaseDagDbAdapter({
      env: opts.env,
      databaseId: opts.databaseId || 'commandbase-main',
      readonly: Boolean(opts.readonly),
    });
  }
  return openSqliteDevDatabase(opts.dbPath, {
    fileMustExist: Boolean(opts.fileMustExist),
    readonly: Boolean(opts.readonly),
  });
}

class CommandBaseReadPool {
  constructor(options) {
    this._connections = [];
    this._size = options.poolSize;
    this._factory = options.connectionFactory;
    for (let i = 0; i < this._size; i += 1) {
      this._connections.push({ conn: this._factory(), busy: false });
    }
  }

  acquire() {
    const slot = this._connections.find((candidate) => !candidate.busy);
    if (!slot) return null;
    slot.busy = true;
    return slot.conn;
  }

  release(conn) {
    const slot = this._connections.find((candidate) => candidate.conn === conn);
    if (slot) slot.busy = false;
  }

  read(fn, fallback) {
    const conn = this.acquire();
    if (conn) {
      try {
        return fn(conn);
      } finally {
        this.release(conn);
      }
    }
    if (fallback) return fn(fallback);
    throw new Error('[commandbase-db-factory] Read pool exhausted and no fallback provided');
  }

  stats() {
    const busy = this._connections.filter((slot) => slot.busy).length;
    return { size: this._size, busy, idle: this._size - busy };
  }

  close() {
    for (const slot of this._connections) {
      if (slot.conn && typeof slot.conn.close === 'function') {
        try { slot.conn.close(); } catch (_) {}
      }
    }
    this._connections = [];
    this._size = 0;
  }
}

function createCommandBaseReadPool(options) {
  const opts = options || {};
  const poolSize = parseInt(String(opts.poolSize || process.env.DB_READ_POOL_SIZE || '4'), 10);
  return new CommandBaseReadPool({
    poolSize,
    connectionFactory: () => createCommandBaseDb({
      env: opts.env,
      dbPath: opts.dbPath,
      databaseId: opts.databaseId || 'commandbase-main',
      fileMustExist: Boolean(opts.fileMustExist),
      readonly: true,
    }),
  });
}

function createTaskForceDb(options) {
  const opts = options || {};
  return createCommandBaseDb({
    env: opts.env,
    dbPath: opts.dbPath,
    databaseId: 'commandbase-task-forces',
    fileMustExist: Boolean(opts.fileMustExist),
    readonly: Boolean(opts.readonly),
  });
}

module.exports = {
  createCommandBaseDb,
  createCommandBaseReadPool,
  createTaskForceDb,
  defaultCommandBaseDbPath,
  defaultTaskForceDbPath,
  productionUsesDagDb,
  requireDagDbConfig,
};
