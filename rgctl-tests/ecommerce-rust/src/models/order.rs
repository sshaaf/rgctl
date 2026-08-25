use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Order {
    pub id: String,
    pub user_id: String,
    pub status: String,
    pub total_cents: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct OrderItem {
    pub id: String,
    pub order_id: String,
    pub product_id: String,
    pub quantity: i64,
    pub unit_price_cents: i64,
}
