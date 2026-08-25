use serde::Serialize;
use crate::models::order::OrderItem;

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub status: String,
    pub total_cents: i64,
    pub items: Vec<OrderItem>,
}
