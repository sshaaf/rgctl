use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Review {
    pub id: String,
    pub product_id: String,
    pub user_id: String,
    pub rating: i64,
    pub comment: String,
    pub created_at: String,
}
