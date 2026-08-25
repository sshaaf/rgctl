use crate::{error::AppResult, models::review::Review};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, r: &Review) -> AppResult<Review> {
    sqlx::query_as::<_, Review>(
        "INSERT INTO reviews (id, product_id, user_id, rating, comment, created_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id, product_id, user_id, rating, comment, created_at",
    )
    .bind(&r.id)
    .bind(&r.product_id)
    .bind(&r.user_id)
    .bind(r.rating)
    .bind(&r.comment)
    .bind(&r.created_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn list_for_product(pool: &SqlitePool, product_id: &str) -> AppResult<Vec<Review>> {
    sqlx::query_as::<_, Review>(
        "SELECT id, product_id, user_id, rating, comment, created_at FROM reviews WHERE product_id = ? ORDER BY created_at DESC",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
