use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CartItem {
    pub user_id: String,
    pub product_id: String,
    pub quantity: i64,
}
