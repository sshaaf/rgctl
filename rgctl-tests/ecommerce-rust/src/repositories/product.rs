use crate::{error::AppResult, models::product::Product};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, p: &Product) -> AppResult<Product> {
    sqlx::query_as::<_, Product>(
        "INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id, category_id, name, slug, description, price_cents, stock, created_at",
    )
    .bind(&p.id)
    .bind(&p.category_id)
    .bind(&p.name)
    .bind(&p.slug)
    .bind(&p.description)
    .bind(p.price_cents)
    .bind(p.stock)
    .bind(&p.created_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Product>> {
    sqlx::query_as::<_, Product>(
        "SELECT id, category_id, name, slug, description, price_cents, stock, created_at FROM products ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Product>> {
    sqlx::query_as::<_, Product>(
        "SELECT id, category_id, name, slug, description, price_cents, stock, created_at FROM products WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}
