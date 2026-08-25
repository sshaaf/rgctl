use uuid::Uuid;
use crate::{
    dto::order::OrderResponse,
    error::{AppError, AppResult},
    models::order::{Order, OrderItem},
    repositories::{cart, inventory, order as repo, product},
    state::AppState,
    utils::time,
};

pub async fn checkout(state: &AppState, user_id: &str) -> AppResult<OrderResponse> {
    let items = cart::list_for_user(&state.pool, user_id).await?;
    if items.is_empty() { return Err(AppError::BadRequest("cart is empty".into())); }

    let order_id = Uuid::new_v4().to_string();
    let mut total = 0i64;
    let mut order_items = Vec::new();

    for item in &items {
        let p = product::find_by_id(&state.pool, &item.product_id).await?.ok_or(AppError::NotFound)?;
        if p.stock < item.quantity {
            return Err(AppError::BadRequest(format!("insufficient stock for {}", p.name)));
        }
        total += p.price_cents * item.quantity;
        order_items.push(OrderItem {
            id: Uuid::new_v4().to_string(),
            order_id: order_id.clone(),
            product_id: p.id.clone(),
            quantity: item.quantity,
            unit_price_cents: p.price_cents,
        });
    }

    let order = Order { id: order_id.clone(), user_id: user_id.into(), status: "confirmed".into(), total_cents: total, created_at: time::now_iso() };
    repo::create(&state.pool, &order).await?;
    for oi in &order_items {
        inventory::decrement_stock(&state.pool, &oi.product_id, oi.quantity).await?;
        repo::add_item(&state.pool, oi).await?;
    }
    cart::clear(&state.pool, user_id).await?;
    Ok(OrderResponse { id: order.id, status: order.status, total_cents: order.total_cents, items: order_items })
}

pub async fn list(state: &AppState, user_id: &str) -> AppResult<Vec<OrderResponse>> {
    let mut out = Vec::new();
    for o in repo::list_for_user(&state.pool, user_id).await? {
        let items = repo::items_for_order(&state.pool, &o.id).await?;
        out.push(OrderResponse { id: o.id, status: o.status, total_cents: o.total_cents, items });
    }
    Ok(out)
}

pub async fn get(state: &AppState, user_id: &str, id: &str) -> AppResult<OrderResponse> {
    let o = repo::find_by_id(&state.pool, id).await?.ok_or(AppError::NotFound)?;
    if o.user_id != user_id { return Err(AppError::Unauthorized); }
    let items = repo::items_for_order(&state.pool, &o.id).await?;
    Ok(OrderResponse { id: o.id, status: o.status, total_cents: o.total_cents, items })
}
