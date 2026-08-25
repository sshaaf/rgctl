mod cart;
mod orders;
mod products;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/services/products", products::routes())
        .nest("/services/cart", cart::routes())
        .nest("/services/orders", orders::routes())
}
