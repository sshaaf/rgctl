use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub category_id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: i64,
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub price_cents: i64,
    pub stock: i64,
}
