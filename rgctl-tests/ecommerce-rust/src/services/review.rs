use uuid::Uuid;
use crate::{
    dto::review::{CreateReviewRequest, ReviewResponse},
    error::{AppError, AppResult},
    models::review::Review,
    repositories::{product, review as repo},
    state::AppState,
    utils::time,
};

pub async fn create(state: &AppState, user_id: &str, product_id: &str, req: CreateReviewRequest) -> AppResult<ReviewResponse> {
    if req.rating < 1 || req.rating > 5 { return Err(AppError::BadRequest("rating must be 1-5".into())); }
    product::find_by_id(&state.pool, product_id).await?.ok_or(AppError::NotFound)?;
    let r = Review { id: Uuid::new_v4().to_string(), product_id: product_id.into(), user_id: user_id.into(), rating: req.rating, comment: req.comment, created_at: time::now_iso() };
    let saved = repo::create(&state.pool, &r).await?;
    Ok(ReviewResponse { id: saved.id, product_id: saved.product_id, user_id: saved.user_id, rating: saved.rating, comment: saved.comment })
}

pub async fn list(state: &AppState, product_id: &str) -> AppResult<Vec<ReviewResponse>> {
    product::find_by_id(&state.pool, product_id).await?.ok_or(AppError::NotFound)?;
    Ok(repo::list_for_product(&state.pool, product_id).await?.into_iter().map(|r| ReviewResponse { id: r.id, product_id: r.product_id, user_id: r.user_id, rating: r.rating, comment: r.comment }).collect())
}
