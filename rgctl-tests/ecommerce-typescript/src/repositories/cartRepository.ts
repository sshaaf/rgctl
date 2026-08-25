import Database from 'better-sqlite3';
import { CartItem } from '../models/cart';

export function listCartItems(db: Database.Database, userId: string): CartItem[] {
  return db
    .prepare('SELECT user_id, product_id, quantity FROM cart_items WHERE user_id = ?')
    .all(userId) as CartItem[];
}

export function upsertCartItem(db: Database.Database, item: CartItem): void {
  db.prepare(
    `INSERT INTO cart_items (user_id, product_id, quantity) VALUES (?, ?, ?)
     ON CONFLICT(user_id, product_id) DO UPDATE SET quantity = excluded.quantity`,
  ).run(item.user_id, item.product_id, item.quantity);
}

export function removeCartItem(
  db: Database.Database,
  userId: string,
  productId: string,
): void {
  db.prepare('DELETE FROM cart_items WHERE user_id = ? AND product_id = ?').run(
    userId,
    productId,
  );
}

export function clearCart(db: Database.Database, userId: string): void {
  db.prepare('DELETE FROM cart_items WHERE user_id = ?').run(userId);
}
