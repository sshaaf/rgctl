use axum::{extract::{Path, State}, routing::get, Json, Router};

use crate::{
    coolstore::model::CatalogProduct,
    error::AppResult,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_all))
        .route("/{item_id}", get(get_product))
}

async fn list_all(State(state): State<AppState>) -> AppResult<Json<Vec<CatalogProduct>>> {
    Ok(Json(state.coolstore.products.get_products()))
}

async fn get_product(
    State(state): State<AppState>,
    Path(item_id): Path<String>,
) -> AppResult<Json<Option<CatalogProduct>>> {
    Ok(Json(state.coolstore.products.get_product_by_item_id(&item_id)))
}
