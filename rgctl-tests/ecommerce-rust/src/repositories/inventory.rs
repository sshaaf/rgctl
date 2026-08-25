use crate::error::{AppError, AppResult};
use sqlx::SqlitePool;

pub async fn decrement_stock(pool: &SqlitePool, product_id: &str, qty: i64) -> AppResult<()> {
    let result = sqlx::query(
        "UPDATE products SET stock = stock - ? WHERE id = ? AND stock >= ?",
    )
    .bind(qty)
    .bind(product_id)
    .bind(qty)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("insufficient stock".into()));
    }
    Ok(())
}
