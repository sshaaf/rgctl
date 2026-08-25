import { v4 as uuidv4 } from 'uuid';
import { getDb } from '../db';
import { Review } from '../models/review';
import * as productRepository from '../repositories/productRepository';
import * as reviewRepository from '../repositories/reviewRepository';
import { AppError } from '../utils/errors';
import { nowIso } from '../utils/time';

export interface CreateReviewRequest {
  rating: number;
  comment: string;
}

export interface ReviewResponse {
  id: string;
  product_id: string;
  user_id: string;
  rating: number;
  comment: string;
}

function toResponse(review: Review): ReviewResponse {
  return {
    id: review.id,
    product_id: review.product_id,
    user_id: review.user_id,
    rating: review.rating,
    comment: review.comment,
  };
}

export function createReview(
  userId: string,
  productId: string,
  req: CreateReviewRequest,
): ReviewResponse {
  const db = getDb();

  if (req.rating < 1 || req.rating > 5) {
    throw AppError.badRequest('rating must be 1-5');
  }

  if (!productRepository.findProductById(db, productId)) {
    throw AppError.notFound();
  }

  const review: Review = {
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

export function listReviews(productId: string): ReviewResponse[] {
  const db = getDb();

  if (!productRepository.findProductById(db, productId)) {
    throw AppError.notFound();
  }

  return reviewRepository.listReviewsForProduct(db, productId).map(toResponse);
}
