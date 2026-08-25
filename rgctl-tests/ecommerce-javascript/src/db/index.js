const { createDatabase, seedDemoData, migrate } = require('./seed');
const { SCHEMA_SQL } = require('./schema');

let dbInstance = null;

function getDb() {
  if (!dbInstance) {
    throw new Error('Database not initialized');
  }
  return dbInstance;
}

function initDb(databasePath) {
  dbInstance = createDatabase(databasePath);
  seedDemoData(dbInstance);
  return dbInstance;
}

function closeDb() {
  if (dbInstance) {
    dbInstance.close();
    dbInstance = null;
  }
}

module.exports = {
  getDb,
  initDb,
  closeDb,
  createDatabase,
  seedDemoData,
  migrate,
  SCHEMA_SQL,
};
