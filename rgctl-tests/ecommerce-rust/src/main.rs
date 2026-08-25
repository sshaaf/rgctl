mod config;
mod coolstore;
mod cfg_features;
mod correctness;
mod db;
mod dto;
mod error;
mod middleware;
mod models;
mod repositories;
mod routes;
mod services;
mod state;
mod utils;

use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> error::AppResult<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::Config::from_env();
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;
    seed_demo(&pool).await?;

    let state = state::AppState {
        pool,
        config: config.clone(),
        coolstore: coolstore::CoolstoreState::new(),
    };
    let app = routes::router().layer(CorsLayer::permissive()).with_state(state);

    let addr: SocketAddr = config.bind_addr.parse().expect("invalid BIND_ADDR");
    tracing::info!(%addr, "ecommerce-rust listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn seed_demo(pool: &sqlx::SqlitePool) -> error::AppResult<()> {
    use uuid::Uuid;
    use utils::time;
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories").fetch_one(pool).await?;
    if count.0 > 0 { return Ok(()); }

    let cat_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO categories (id, name, slug, created_at) VALUES (?, ?, ?, ?)")
        .bind(&cat_id).bind("Electronics").bind("electronics").bind(time::now_iso())
        .execute(pool).await?;

    sqlx::query("INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string()).bind(&cat_id).bind("Wireless Headphones").bind("wireless-headphones")
        .bind("Noise cancelling over-ear headphones").bind(12999_i64).bind(50_i64).bind(time::now_iso())
        .execute(pool).await?;

    sqlx::query("INSERT INTO products (id, category_id, name, slug, description, price_cents, stock, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4().to_string()).bind(&cat_id).bind("USB-C Hub").bind("usb-c-hub")
        .bind("7-in-1 adapter").bind(4999_i64).bind(120_i64).bind(time::now_iso())
        .execute(pool).await?;
    Ok(())
}
