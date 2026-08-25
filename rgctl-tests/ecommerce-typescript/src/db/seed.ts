import Database from 'better-sqlite3';
import { v4 as uuidv4 } from 'uuid';
import { nowIso } from '../utils/time';
import { SCHEMA_SQL } from './schema';

export function migrate(db: Database.Database): void {
  db.exec(SCHEMA_SQL);
}

export function createDatabase(databasePath: string): Database.Database {
  const db = new Database(databasePath);
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  migrate(db);
  return db;
}

export function seedDemoData(db: Database.Database): void {
  const row = db.prepare('SELECT COUNT(*) AS count FROM categories').get() as { count: number };
  if (row.count > 0) {
    return;
  }

  const catId = uuidv4();
  const now = nowIso();

  db.prepare(
    'INSERT INTO categories (id, name, slug, created_at) VALUES (?, ?, ?, ?)',
  ).run(catId, 'Electronics', 'electronics', now);

  db.prepare(
    `INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    uuidv4(),
    catId,
    'Wireless Headphones',
    'wireless-headphones',
    'Noise cancelling over-ear headphones',
    12999,
    50,
    now,
  );

  db.prepare(
    `INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    uuidv4(),
    catId,
    'USB-C Hub',
    'usb-c-hub',
    '7-in-1 adapter',
    4999,
    120,
    now,
  );
}
