use axum::{routing::{get, post}, extract::Path, Json, Router};
use crate::{dto::category::{CategoryResponse, CreateCategoryRequest}, error::AppResult, middleware::auth::AuthUser, services::category, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list).post(create)).route("/{id}", get(get_one))
}

async fn list(state: axum::extract::State<AppState>) -> AppResult<Json<Vec<CategoryResponse>>> {
    Ok(Json(category::list(&state).await?))
}

async fn create(_user: AuthUser, state: axum::extract::State<AppState>, Json(body): Json<CreateCategoryRequest>) -> AppResult<Json<CategoryResponse>> {
    Ok(Json(category::create(&state, body).await?))
}

async fn get_one(state: axum::extract::State<AppState>, Path(id): Path<String>) -> AppResult<Json<CategoryResponse>> {
    Ok(Json(category::get(&state, &id).await?))
}
