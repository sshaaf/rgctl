use crate::error::AppResult;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

pub async fn connect(database_url: &str) -> AppResult<SqlitePool> {
    let url = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{url}?mode=rwc"))
        .await?;
    Ok(pool)
}

pub async fn migrate(pool: &SqlitePool) -> AppResult<()> {
    let sql = include_str!("../migrations/001_init.sql");
    for stmt in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        sqlx::query(stmt).execute(pool).await?;
    }
    Ok(())
}
