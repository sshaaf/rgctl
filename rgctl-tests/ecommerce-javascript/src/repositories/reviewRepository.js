function createReview(db, review) {
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

function listReviewsForProduct(db, productId) {
  return db
    .prepare(
      `SELECT id, product_id, user_id, rating, comment, created_at
       FROM reviews WHERE product_id = ? ORDER BY created_at DESC`,
    )
    .all(productId);
}

module.exports = { createReview, listReviewsForProduct };
