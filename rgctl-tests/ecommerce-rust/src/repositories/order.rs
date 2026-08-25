use crate::{
    error::AppResult,
    models::order::{Order, OrderItem},
};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, order: &Order) -> AppResult<Order> {
    sqlx::query_as::<_, Order>(
        "INSERT INTO orders (id, user_id, status, total_cents, created_at) VALUES (?, ?, ?, ?, ?) RETURNING id, user_id, status, total_cents, created_at",
    )
    .bind(&order.id)
    .bind(&order.user_id)
    .bind(&order.status)
    .bind(order.total_cents)
    .bind(&order.created_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn add_item(pool: &SqlitePool, item: &OrderItem) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO order_items (id, order_id, product_id, quantity, unit_price_cents) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&item.id)
    .bind(&item.order_id)
    .bind(&item.product_id)
    .bind(item.quantity)
    .bind(item.unit_price_cents)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_for_user(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<Order>> {
    sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, total_cents, created_at FROM orders WHERE user_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Order>> {
    sqlx::query_as::<_, Order>(
        "SELECT id, user_id, status, total_cents, created_at FROM orders WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn items_for_order(pool: &SqlitePool, order_id: &str) -> AppResult<Vec<OrderItem>> {
    sqlx::query_as::<_, OrderItem>(
        "SELECT id, order_id, product_id, quantity, unit_price_cents FROM order_items WHERE order_id = ?",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}
