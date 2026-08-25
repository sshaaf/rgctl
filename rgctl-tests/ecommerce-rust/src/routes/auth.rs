use axum::{routing::post, Json, Router};
use crate::{dto::auth::{AuthResponse, LoginRequest, RegisterRequest}, error::AppResult, services::auth, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/register", post(register)).route("/login", post(login))
}

async fn register(state: axum::extract::State<AppState>, Json(body): Json<RegisterRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(auth::register(&state, body).await?))
}

async fn login(state: axum::extract::State<AppState>, Json(body): Json<LoginRequest>) -> AppResult<Json<AuthResponse>> {
    Ok(Json(auth::login(&state, body).await?))
}
