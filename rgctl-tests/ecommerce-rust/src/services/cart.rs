use crate::{
    dto::cart::{AddCartItemRequest, CartItemResponse},
    error::{AppError, AppResult},
    models::cart::CartItem,
    repositories::{cart as repo, product},
    state::AppState,
};

pub async fn list(state: &AppState, user_id: &str) -> AppResult<Vec<CartItemResponse>> {
    Ok(repo::list_for_user(&state.pool, user_id).await?.into_iter().map(|i| CartItemResponse { product_id: i.product_id, quantity: i.quantity }).collect())
}

pub async fn add(state: &AppState, user_id: &str, req: AddCartItemRequest) -> AppResult<CartItemResponse> {
    if req.quantity <= 0 { return Err(AppError::BadRequest("quantity must be positive".into())); }
    product::find_by_id(&state.pool, &req.product_id).await?.ok_or(AppError::NotFound)?;
    let item = CartItem { user_id: user_id.into(), product_id: req.product_id.clone(), quantity: req.quantity };
    repo::upsert(&state.pool, &item).await?;
    Ok(CartItemResponse { product_id: req.product_id, quantity: req.quantity })
}

pub async fn remove(state: &AppState, user_id: &str, product_id: &str) -> AppResult<()> {
    repo::remove(&state.pool, user_id, product_id).await
}
