function decrementStock(db, productId, quantity) {
  db.prepare('UPDATE products SET stock = stock - ? WHERE id = ?').run(quantity, productId);
}

module.exports = { decrementStock };
