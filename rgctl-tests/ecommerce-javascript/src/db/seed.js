const { v4: uuidv4 } = require('uuid');
const { nowIso } = require('../utils/time');
const { SCHEMA_SQL } = require('./schema');

function migrate(db) {
  db.exec(SCHEMA_SQL);
}

function createDatabase(databasePath) {
  const Database = require('better-sqlite3');
  const db = new Database(databasePath);
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  migrate(db);
  return db;
}

function seedDemoData(db) {
  const row = db.prepare('SELECT COUNT(*) AS count FROM categories').get();
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

module.exports = { migrate, createDatabase, seedDemoData };
