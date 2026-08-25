use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: i64,
    pub created_at: String,
}
