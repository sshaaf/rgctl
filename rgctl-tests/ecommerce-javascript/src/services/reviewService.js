const { v4: uuidv4 } = require('uuid');
const { getDb } = require('../db');
const productRepository = require('../repositories/productRepository');
const reviewRepository = require('../repositories/reviewRepository');
const { AppError } = require('../utils/errors');
const { nowIso } = require('../utils/time');

function toResponse(review) {
  return {
    id: review.id,
    product_id: review.product_id,
    user_id: review.user_id,
    rating: review.rating,
    comment: review.comment,
  };
}

function createReview(userId, productId, req) {
  const db = getDb();

  if (req.rating < 1 || req.rating > 5) {
    throw AppError.badRequest('rating must be 1-5');
  }

  if (!productRepository.findProductById(db, productId)) {
    throw AppError.notFound();
  }

  const review = {
    id: uuidv4(),
    product_id: productId,
    user_id: userId,
    rating: req.rating,
    comment: req.comment,
    created_at: nowIso(),
  };

  reviewRepository.createReview(db, review);
  return toResponse(review);
}

function listReviews(productId) {
  const db = getDb();

  if (!productRepository.findProductById(db, productId)) {
    throw AppError.notFound();
  }

  return reviewRepository.listReviewsForProduct(db, productId).map(toResponse);
}

module.exports = { createReview, listReviews };
