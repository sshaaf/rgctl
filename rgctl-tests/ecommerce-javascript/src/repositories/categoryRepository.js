function createCategory(db, category) {
  db.prepare(
    'INSERT INTO categories (id, name, slug, created_at) VALUES (?, ?, ?, ?)',
  ).run(category.id, category.name, category.slug, category.created_at);
  return category;
}

function listCategories(db) {
  return db
    .prepare('SELECT id, name, slug, created_at FROM categories ORDER BY name')
    .all();
}

function findCategoryById(db, id) {
  return db
    .prepare('SELECT id, name, slug, created_at FROM categories WHERE id = ?')
    .get(id);
}

module.exports = { createCategory, listCategories, findCategoryById };
