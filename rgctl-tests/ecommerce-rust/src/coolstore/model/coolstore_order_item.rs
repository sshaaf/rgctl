use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolstoreOrderItem {
    pub product_id: String,
    pub quantity: i32,
    pub price: f64,
}

impl CoolstoreOrderItem {
    pub fn new(product_id: impl Into<String>, quantity: i32, price: f64) -> Self {
        Self {
            product_id: product_id.into(),
            quantity,
            price,
        }
    }
}
