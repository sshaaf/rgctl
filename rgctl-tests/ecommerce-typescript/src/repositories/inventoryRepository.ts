import Database from 'better-sqlite3';

export function decrementStock(
  db: Database.Database,
  productId: string,
  quantity: number,
): void {
  db.prepare('UPDATE products SET stock = stock - ? WHERE id = ?').run(quantity, productId);
}
