import Database from 'better-sqlite3';
import { Category } from '../models/category';

export function createCategory(db: Database.Database, category: Category): Category {
  db.prepare(
    'INSERT INTO categories (id, name, slug, created_at) VALUES (?, ?, ?, ?)',
  ).run(category.id, category.name, category.slug, category.created_at);
  return category;
}

export function listCategories(db: Database.Database): Category[] {
  return db
    .prepare('SELECT id, name, slug, created_at FROM categories ORDER BY name')
    .all() as Category[];
}

export function findCategoryById(db: Database.Database, id: string): Category | undefined {
  return db
    .prepare('SELECT id, name, slug, created_at FROM categories WHERE id = ?')
    .get(id) as Category | undefined;
}
