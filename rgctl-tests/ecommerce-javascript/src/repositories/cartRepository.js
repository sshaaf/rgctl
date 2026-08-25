function listCartItems(db, userId) {
  return db
    .prepare('SELECT user_id, product_id, quantity FROM cart_items WHERE user_id = ?')
    .all(userId);
}

function upsertCartItem(db, item) {
  db.prepare(
    `INSERT INTO cart_items (user_id, product_id, quantity) VALUES (?, ?, ?)
     ON CONFLICT(user_id, product_id) DO UPDATE SET quantity = excluded.quantity`,
  ).run(item.user_id, item.product_id, item.quantity);
}

function removeCartItem(db, userId, productId) {
  db.prepare('DELETE FROM cart_items WHERE user_id = ? AND product_id = ?').run(
    userId,
    productId,
  );
}

function clearCart(db, userId) {
  db.prepare('DELETE FROM cart_items WHERE user_id = ?').run(userId);
}

module.exports = { listCartItems, upsertCartItem, removeCartItem, clearCart };
