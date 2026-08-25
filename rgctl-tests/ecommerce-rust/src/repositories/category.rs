use crate::{error::AppResult, models::category::Category};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, c: &Category) -> AppResult<Category> {
    sqlx::query_as::<_, Category>(
        "INSERT INTO categories (id, name, slug, created_at) VALUES (?, ?, ?, ?) RETURNING id, name, slug, created_at",
    )
    .bind(&c.id)
    .bind(&c.name)
    .bind(&c.slug)
    .bind(&c.created_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Category>> {
    sqlx::query_as::<_, Category>("SELECT id, name, slug, created_at FROM categories ORDER BY name")
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Category>> {
    sqlx::query_as::<_, Category>("SELECT id, name, slug, created_at FROM categories WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Into::into)
}
