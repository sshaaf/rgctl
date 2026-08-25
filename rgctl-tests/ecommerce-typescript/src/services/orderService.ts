import { v4 as uuidv4 } from 'uuid';
import { getDb } from '../db';
import { Order, OrderItem } from '../models/order';
import * as cartRepository from '../repositories/cartRepository';
import * as inventoryRepository from '../repositories/inventoryRepository';
import * as orderRepository from '../repositories/orderRepository';
import * as productRepository from '../repositories/productRepository';
import { AppError } from '../utils/errors';
import { nowIso } from '../utils/time';

export interface OrderResponse {
  id: string;
  status: string;
  total_cents: number;
  items: OrderItem[];
}

function toResponse(order: Order, items: OrderItem[]): OrderResponse {
  return {
    id: order.id,
    status: order.status,
    total_cents: order.total_cents,
    items,
  };
}

export function checkout(userId: string): OrderResponse {
  const db = getDb();
  const cartItems = cartRepository.listCartItems(db, userId);

  if (cartItems.length === 0) {
    throw AppError.badRequest('cart is empty');
  }

  const orderId = uuidv4();
  let total = 0;
  const orderItems: OrderItem[] = [];

  for (const item of cartItems) {
    const product = productRepository.findProductById(db, item.product_id);
    if (!product) {
      throw AppError.notFound();
    }
    if (product.stock < item.quantity) {
      throw AppError.badRequest(`insufficient stock for ${product.name}`);
    }
    total += product.price_cents * item.quantity;
    orderItems.push({
      id: uuidv4(),
      order_id: orderId,
      product_id: product.id,
      quantity: item.quantity,
      unit_price_cents: product.price_cents,
    });
  }

  const order: Order = {
    id: orderId,
    user_id: userId,
    status: 'confirmed',
    total_cents: total,
    created_at: nowIso(),
  };

  orderRepository.createOrder(db, order);

  for (const item of orderItems) {
    inventoryRepository.decrementStock(db, item.product_id, item.quantity);
    orderRepository.addOrderItem(db, item);
  }

  cartRepository.clearCart(db, userId);
  return toResponse(order, orderItems);
}

export function listOrders(userId: string): OrderResponse[] {
  const db = getDb();
  return orderRepository.listOrdersForUser(db, userId).map((order) => {
    const items = orderRepository.listOrderItems(db, order.id);
    return toResponse(order, items);
  });
}

export function getOrder(userId: string, id: string): OrderResponse {
  const db = getDb();
  const order = orderRepository.findOrderById(db, id);

  if (!order) {
    throw AppError.notFound();
  }
  if (order.user_id !== userId) {
    throw AppError.unauthorized();
  }

  const items = orderRepository.listOrderItems(db, order.id);
  return toResponse(order, items);
}
