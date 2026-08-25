function createProduct(db, product) {
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

function listProducts(db) {
  return db
    .prepare(
      `SELECT id, category_id, name, slug, description, price_cents, stock, created_at
       FROM products ORDER BY name`,
    )
    .all();
}

function findProductById(db, id) {
  return db
    .prepare(
      `SELECT id, category_id, name, slug, description, price_cents, stock, created_at
       FROM products WHERE id = ?`,
    )
    .get(id);
}

module.exports = { createProduct, listProducts, findProductById };
