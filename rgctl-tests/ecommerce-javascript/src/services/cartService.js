const { getDb } = require('../db');
const cartRepository = require('../repositories/cartRepository');
const productRepository = require('../repositories/productRepository');
const { AppError } = require('../utils/errors');

function listCart(userId) {
  const db = getDb();
  return cartRepository.listCartItems(db, userId).map((item) => ({
    product_id: item.product_id,
    quantity: item.quantity,
  }));
}

function addCartItem(userId, req) {
  const db = getDb();

  if (req.quantity <= 0) {
    throw AppError.badRequest('quantity must be positive');
  }

  if (!productRepository.findProductById(db, req.product_id)) {
    throw AppError.notFound();
  }

  const item = {
    user_id: userId,
    product_id: req.product_id,
    quantity: req.quantity,
  };

  cartRepository.upsertCartItem(db, item);
  return { product_id: req.product_id, quantity: req.quantity };
}

function removeCartItem(userId, productId) {
  const db = getDb();
  cartRepository.removeCartItem(db, userId, productId);
}

module.exports = { listCart, addCartItem, removeCartItem };
