use uuid::Uuid;
use crate::{
    dto::product::{CreateProductRequest, ProductResponse},
    error::{AppError, AppResult},
    models::product::Product,
    repositories::{category, product as repo},
    state::AppState,
    utils::time,
};

pub async fn create(state: &AppState, req: CreateProductRequest) -> AppResult<ProductResponse> {
    if category::find_by_id(&state.pool, &req.category_id).await?.is_none() {
        return Err(AppError::BadRequest("unknown category".into()));
    }
    let p = Product {
        id: Uuid::new_v4().to_string(),
        category_id: req.category_id,
        name: req.name,
        slug: req.slug,
        description: req.description,
        price_cents: req.price_cents,
        stock: req.stock,
        created_at: time::now_iso(),
    };
    let saved = repo::create(&state.pool, &p).await?;
    Ok(to_response(saved))
}

pub async fn list(state: &AppState) -> AppResult<Vec<ProductResponse>> {
    Ok(repo::list(&state.pool).await?.into_iter().map(to_response).collect())
}

pub async fn get(state: &AppState, id: &str) -> AppResult<ProductResponse> {
    let p = repo::find_by_id(&state.pool, id).await?.ok_or(AppError::NotFound)?;
    Ok(to_response(p))
}

fn to_response(p: Product) -> ProductResponse {
    ProductResponse {
        id: p.id, category_id: p.category_id, name: p.name, slug: p.slug,
        description: p.description, price_cents: p.price_cents, stock: p.stock,
    }
}
