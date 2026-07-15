'use strict';

const {
  createCommandBaseDb,
  defaultCommandBaseDbPath,
} = require('./commandbase-db-factory');
const path = require('node:path');

const DB_PATH = process.env.DB_PATH || defaultCommandBaseDbPath(path.join(__dirname, '..'));

let db;
try {
  db = createCommandBaseDb({
    dbPath: DB_PATH,
    databaseId: 'commandbase-main',
    fileMustExist: true,
  });
  console.log(`Connected to CommandBase persistence adapter at ${DB_PATH}`);
} catch (err) {
  console.error('Failed to open CommandBase persistence adapter:', err.message);
  process.exit(1);
}

function localNow() {
  return new Date().toLocaleString('sv-SE', { timeZone: Intl.DateTimeFormat().resolvedOptions().timeZone }).replace(' ', 'T').slice(0, 19).replace('T', ' ');
}

module.exports = { db, DB_PATH, localNow };
