use uuid::Uuid;
use crate::{
    dto::category::{CategoryResponse, CreateCategoryRequest},
    error::{AppError, AppResult},
    models::category::Category,
    repositories::category as repo,
    state::AppState,
    utils::time,
};

pub async fn create(state: &AppState, req: CreateCategoryRequest) -> AppResult<CategoryResponse> {
    let c = Category { id: Uuid::new_v4().to_string(), name: req.name, slug: req.slug, created_at: time::now_iso() };
    let saved = repo::create(&state.pool, &c).await?;
    Ok(CategoryResponse { id: saved.id, name: saved.name, slug: saved.slug })
}

pub async fn list(state: &AppState) -> AppResult<Vec<CategoryResponse>> {
    Ok(repo::list(&state.pool).await?.into_iter().map(|c| CategoryResponse { id: c.id, name: c.name, slug: c.slug }).collect())
}

pub async fn get(state: &AppState, id: &str) -> AppResult<CategoryResponse> {
    let c = repo::find_by_id(&state.pool, id).await?.ok_or(AppError::NotFound)?;
    Ok(CategoryResponse { id: c.id, name: c.name, slug: c.slug })
}
