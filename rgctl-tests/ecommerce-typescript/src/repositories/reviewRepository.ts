import Database from 'better-sqlite3';
import { Review } from '../models/review';

export function createReview(db: Database.Database, review: Review): Review {
  db.prepare(
    `INSERT INTO reviews (id, product_id, user_id, rating, comment, created_at)
     VALUES (?, ?, ?, ?, ?, ?)`,
  ).run(
    review.id,
    review.product_id,
    review.user_id,
    review.rating,
    review.comment,
    review.created_at,
  );
  return review;
}

export function listReviewsForProduct(
  db: Database.Database,
  productId: string,
): Review[] {
  return db
    .prepare(
      `SELECT id, product_id, user_id, rating, comment, created_at
       FROM reviews WHERE product_id = ? ORDER BY created_at DESC`,
    )
    .all(productId) as Review[];
}
