use axum::{routing::{get, post}, extract::Path, Json, Router};
use crate::{dto::order::OrderResponse, error::AppResult, middleware::auth::AuthUser, services::order, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list).post(checkout)).route("/{id}", get(get_one))
}

async fn checkout(user: AuthUser, state: axum::extract::State<AppState>) -> AppResult<Json<OrderResponse>> {
    Ok(Json(order::checkout(&state, &user.user_id).await?))
}

async fn list(user: AuthUser, state: axum::extract::State<AppState>) -> AppResult<Json<Vec<OrderResponse>>> {
    Ok(Json(order::list(&state, &user.user_id).await?))
}

async fn get_one(user: AuthUser, state: axum::extract::State<AppState>, Path(id): Path<String>) -> AppResult<Json<OrderResponse>> {
    Ok(Json(order::get(&state, &user.user_id, &id).await?))
}
