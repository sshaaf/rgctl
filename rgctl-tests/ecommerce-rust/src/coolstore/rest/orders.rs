use axum::{extract::{Path, State}, routing::get, Json, Router};

use crate::{
    coolstore::model::CoolstoreOrder,
    error::{AppError, AppResult},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_all))
        .route("/{order_id}", get(get_order))
}

async fn list_all(State(state): State<AppState>) -> AppResult<Json<Vec<CoolstoreOrder>>> {
    Ok(Json(state.coolstore.orders.get_orders()))
}

async fn get_order(
    State(state): State<AppState>,
    Path(order_id): Path<i64>,
) -> AppResult<Json<CoolstoreOrder>> {
    state
        .coolstore
        .orders
        .get_order_by_id(order_id)
        .map(Json)
        .ok_or(AppError::NotFound)
}
