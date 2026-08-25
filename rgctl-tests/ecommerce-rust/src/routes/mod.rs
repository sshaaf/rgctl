pub mod auth;
pub mod cart;
pub mod categories;
pub mod health;
pub mod orders;
pub mod products;
pub mod reviews;

use axum::{routing::{get, post, delete}, Router};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .nest("/api/auth", auth::routes())
        .nest("/api/categories", categories::routes())
        .nest("/api/products", products::routes())
        .nest("/api/cart", cart::routes())
        .nest("/api/orders", orders::routes())
        .merge(crate::coolstore::rest::routes())
}
