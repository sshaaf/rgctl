const { v4: uuidv4 } = require('uuid');
const { getDb } = require('../db');
const categoryRepository = require('../repositories/categoryRepository');
const productRepository = require('../repositories/productRepository');
const { AppError } = require('../utils/errors');
const { nowIso } = require('../utils/time');

function toResponse(product) {
  return {
    id: product.id,
    category_id: product.category_id,
    name: product.name,
    slug: product.slug,
    description: product.description,
    price_cents: product.price_cents,
    stock: product.stock,
  };
}

function createProduct(req) {
  const db = getDb();

  if (!categoryRepository.findCategoryById(db, req.category_id)) {
    throw AppError.badRequest('unknown category');
  }

  const product = {
    id: uuidv4(),
    category_id: req.category_id,
    name: req.name,
    slug: req.slug,
    description: req.description,
    price_cents: req.price_cents,
    stock: req.stock,
    created_at: nowIso(),
  };

  productRepository.createProduct(db, product);
  return toResponse(product);
}

function listProducts() {
  const db = getDb();
  return productRepository.listProducts(db).map(toResponse);
}

function getProduct(id) {
  const db = getDb();
  const product = productRepository.findProductById(db, id);
  if (!product) {
    throw AppError.notFound();
  }
  return toResponse(product);
}

module.exports = { createProduct, listProducts, getProduct };
