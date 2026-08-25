import Database from 'better-sqlite3';
import { Order, OrderItem } from '../models/order';

export function createOrder(db: Database.Database, order: Order): Order {
  db.prepare(
    'INSERT INTO orders (id, user_id, status, total_cents, created_at) VALUES (?, ?, ?, ?, ?)',
  ).run(order.id, order.user_id, order.status, order.total_cents, order.created_at);
  return order;
}

export function addOrderItem(db: Database.Database, item: OrderItem): void {
  db.prepare(
    `INSERT INTO order_items (id, order_id, product_id, quantity, unit_price_cents)
     VALUES (?, ?, ?, ?, ?)`,
  ).run(item.id, item.order_id, item.product_id, item.quantity, item.unit_price_cents);
}

export function listOrdersForUser(db: Database.Database, userId: string): Order[] {
  return db
    .prepare(
      `SELECT id, user_id, status, total_cents, created_at
       FROM orders WHERE user_id = ? ORDER BY created_at DESC`,
    )
    .all(userId) as Order[];
}

export function findOrderById(db: Database.Database, id: string): Order | undefined {
  return db
    .prepare(
      'SELECT id, user_id, status, total_cents, created_at FROM orders WHERE id = ?',
    )
    .get(id) as Order | undefined;
}

export function listOrderItems(db: Database.Database, orderId: string): OrderItem[] {
  return db
    .prepare(
      `SELECT id, order_id, product_id, quantity, unit_price_cents
       FROM order_items WHERE order_id = ?`,
    )
    .all(orderId) as OrderItem[];
}
