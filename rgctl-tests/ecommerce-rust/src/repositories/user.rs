use crate::{error::AppResult, models::user::User};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, user: &User) -> AppResult<User> {
    sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash, name, role, created_at) VALUES (?, ?, ?, ?, ?, ?) RETURNING id, email, password_hash, name, role, created_at",
    )
    .bind(&user.id)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.name)
    .bind(&user.role)
    .bind(&user.created_at)
    .fetch_one(pool)
    .await
    .map_err(Into::into)
}

pub async fn find_by_email(pool: &SqlitePool, email: &str) -> AppResult<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, role, created_at FROM users WHERE email = ?",
    )
    .bind(email)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}

pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<User>> {
    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, role, created_at FROM users WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(Into::into)
}
