use axum::{routing::{get, post}, extract::Path, Json, Router};
use crate::{dto::product::{CreateProductRequest, ProductResponse}, error::AppResult, middleware::auth::AuthUser, routes::reviews, services::product, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one))
        .nest("/{id}/reviews", reviews::routes())
}

async fn list(state: axum::extract::State<AppState>) -> AppResult<Json<Vec<ProductResponse>>> {
    Ok(Json(product::list(&state).await?))
}

async fn create(_user: AuthUser, state: axum::extract::State<AppState>, Json(body): Json<CreateProductRequest>) -> AppResult<Json<ProductResponse>> {
    Ok(Json(product::create(&state, body).await?))
}

async fn get_one(state: axum::extract::State<AppState>, Path(id): Path<String>) -> AppResult<Json<ProductResponse>> {
    Ok(Json(product::get(&state, &id).await?))
}
