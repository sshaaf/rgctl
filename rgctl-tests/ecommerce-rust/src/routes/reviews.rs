use axum::{routing::{get, post}, extract::Path, Json, Router};
use crate::{dto::review::{CreateReviewRequest, ReviewResponse}, error::AppResult, middleware::auth::AuthUser, services::review, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list).post(create))
}

async fn list(state: axum::extract::State<AppState>, Path(id): Path<String>) -> AppResult<Json<Vec<ReviewResponse>>> {
    Ok(Json(review::list(&state, &id).await?))
}

async fn create(user: AuthUser, state: axum::extract::State<AppState>, Path(id): Path<String>, Json(body): Json<CreateReviewRequest>) -> AppResult<Json<ReviewResponse>> {
    Ok(Json(review::create(&state, &user.user_id, &id, body).await?))
}
