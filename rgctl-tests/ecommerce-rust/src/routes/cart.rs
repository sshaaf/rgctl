use axum::{routing::{delete, get, post}, extract::Path, Json, Router};
use crate::{dto::cart::{AddCartItemRequest, CartItemResponse}, error::AppResult, middleware::auth::AuthUser, services::cart, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list)).route("/items", post(add)).route("/items/{product_id}", delete(remove))
}

async fn list(user: AuthUser, state: axum::extract::State<AppState>) -> AppResult<Json<Vec<CartItemResponse>>> {
    Ok(Json(cart::list(&state, &user.user_id).await?))
}

async fn add(user: AuthUser, state: axum::extract::State<AppState>, Json(body): Json<AddCartItemRequest>) -> AppResult<Json<CartItemResponse>> {
    Ok(Json(cart::add(&state, &user.user_id, body).await?))
}

async fn remove(user: AuthUser, state: axum::extract::State<AppState>, Path(product_id): Path<String>) -> AppResult<()> {
    cart::remove(&state, &user.user_id, &product_id).await
}
