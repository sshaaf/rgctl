import Database from 'better-sqlite3';
import { Product } from '../models/product';

export function createProduct(db: Database.Database, product: Product): Product {
  db.prepare(
    `INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    product.id,
    product.category_id,
    product.name,
    product.slug,
    product.description,
    product.price_cents,
    product.stock,
    product.created_at,
  );
  return product;
}

export function listProducts(db: Database.Database): Product[] {
  return db
    .prepare(
      `SELECT id, category_id, name, slug, description, price_cents, stock, created_at
       FROM products ORDER BY name`,
    )
    .all() as Product[];
}

export function findProductById(db: Database.Database, id: string): Product | undefined {
  return db
    .prepare(
      `SELECT id, category_id, name, slug, description, price_cents, stock, created_at
       FROM products WHERE id = ?`,
    )
    .get(id) as Product | undefined;
}
