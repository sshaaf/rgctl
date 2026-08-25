use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AddCartItemRequest {
    pub product_id: String,
    pub quantity: i64,
}

#[derive(Debug, Serialize)]
pub struct CartItemResponse {
    pub product_id: String,
    pub quantity: i64,
}
