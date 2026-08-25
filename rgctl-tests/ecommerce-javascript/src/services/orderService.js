const { v4: uuidv4 } = require('uuid');
const { getDb } = require('../db');
const cartRepository = require('../repositories/cartRepository');
const inventoryRepository = require('../repositories/inventoryRepository');
const orderRepository = require('../repositories/orderRepository');
const productRepository = require('../repositories/productRepository');
const { AppError } = require('../utils/errors');
const { nowIso } = require('../utils/time');

function toResponse(order, items) {
  return {
    id: order.id,
    status: order.status,
    total_cents: order.total_cents,
    items,
  };
}

function checkout(userId) {
  const db = getDb();
  const cartItems = cartRepository.listCartItems(db, userId);

  if (cartItems.length === 0) {
    throw AppError.badRequest('cart is empty');
  }

  const orderId = uuidv4();
  let total = 0;
  const orderItems = [];

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

  const order = {
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

function listOrders(userId) {
  const db = getDb();
  return orderRepository.listOrdersForUser(db, userId).map((order) => {
    const items = orderRepository.listOrderItems(db, order.id);
    return toResponse(order, items);
  });
}

function getOrder(userId, id) {
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

module.exports = { checkout, listOrders, getOrder };
