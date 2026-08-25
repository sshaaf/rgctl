use crate::{error::AppResult, models::cart::CartItem};
use sqlx::SqlitePool;

pub async fn list_for_user(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<CartItem>> {
    sqlx::query_as::<_, CartItem>(
        "SELECT user_id, product_id, quantity FROM cart_items WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn upsert(pool: &SqlitePool, item: &CartItem) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO cart_items (user_id, product_id, quantity) VALUES (?, ?, ?) ON CONFLICT(user_id, product_id) DO UPDATE SET quantity = excluded.quantity",
    )
    .bind(&item.user_id)
    .bind(&item.product_id)
    .bind(item.quantity)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn remove(pool: &SqlitePool, user_id: &str, product_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM cart_items WHERE user_id = ? AND product_id = ?")
        .bind(user_id)
        .bind(product_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn clear(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM cart_items WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}
