import Database from 'better-sqlite3';
import { createDatabase, seedDemoData } from './seed';

export { createDatabase, seedDemoData, migrate } from './seed';
export { SCHEMA_SQL } from './schema';

let dbInstance: Database.Database | null = null;

export function getDb(): Database.Database {
  if (!dbInstance) {
    throw new Error('Database not initialized');
  }
  return dbInstance;
}

export function initDb(databasePath: string): Database.Database {
  dbInstance = createDatabase(databasePath);
  seedDemoData(dbInstance);
  return dbInstance;
}

export function closeDb(): void {
  if (dbInstance) {
    dbInstance.close();
    dbInstance = null;
  }
}
