use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use crate::{
    coolstore::model::ShoppingCart,
    error::AppResult,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/{cart_id}", get(get_cart))
        .route("/checkout/{cart_id}", post(checkout))
        .route("/{cart_id}/{item_id}/{quantity}", post(add).delete(delete_item))
}

async fn get_cart(
    State(state): State<AppState>,
    Path(cart_id): Path<String>,
) -> AppResult<Json<ShoppingCart>> {
    Ok(Json(state.coolstore.carts.get_shopping_cart(&cart_id)))
}

async fn checkout(
    State(state): State<AppState>,
    Path(cart_id): Path<String>,
) -> AppResult<Json<ShoppingCart>> {
    Ok(Json(state.coolstore.carts.check_out_shopping_cart(&cart_id)))
}

async fn add(
    State(state): State<AppState>,
    Path((cart_id, item_id, quantity)): Path<(String, String, i32)>,
) -> AppResult<Json<ShoppingCart>> {
    Ok(Json(state.coolstore.carts.add_item(&cart_id, &item_id, quantity)))
}

async fn delete_item(
    State(state): State<AppState>,
    Path((cart_id, item_id, quantity)): Path<(String, String, i32)>,
) -> AppResult<Json<ShoppingCart>> {
    Ok(Json(
        state
            .coolstore
            .carts
            .delete_item(&cart_id, &item_id, quantity),
    ))
}