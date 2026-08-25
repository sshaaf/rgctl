function createOrder(db, order) {
  db.prepare(
    'INSERT INTO orders (id, user_id, status, total_cents, created_at) VALUES (?, ?, ?, ?, ?)',
  ).run(order.id, order.user_id, order.status, order.total_cents, order.created_at);
  return order;
}

function addOrderItem(db, item) {
  db.prepare(
    `INSERT INTO order_items (id, order_id, product_id, quantity, unit_price_cents)
     VALUES (?, ?, ?, ?, ?)`,
  ).run(item.id, item.order_id, item.product_id, item.quantity, item.unit_price_cents);
}

function listOrdersForUser(db, userId) {
  return db
    .prepare(
      `SELECT id, user_id, status, total_cents, created_at
       FROM orders WHERE user_id = ? ORDER BY created_at DESC`,
    )
    .all(userId);
}

function findOrderById(db, id) {
  return db
    .prepare(
      'SELECT id, user_id, status, total_cents, created_at FROM orders WHERE id = ?',
    )
    .get(id);
}

function listOrderItems(db, orderId) {
  return db
    .prepare(
      `SELECT id, order_id, product_id, quantity, unit_price_cents
       FROM order_items WHERE order_id = ?`,
    )
    .all(orderId);
}

module.exports = {
  createOrder,
  addOrderItem,
  listOrdersForUser,
  findOrderById,
  listOrderItems,
};
