use serde::{Deserialize, Serialize};

use super::CoolstoreOrderItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoolstoreOrder {
    pub order_id: i64,
    pub cart_id: String,
    pub cart_total: f64,
    pub items: Vec<CoolstoreOrderItem>,
}

impl CoolstoreOrder {
    pub fn new() -> Self {
        Self {
            order_id: 0,
            cart_id: String::new(),
            cart_total: 0.0,
            items: Vec::new(),
        }
    }
}

impl Default for CoolstoreOrder {
    fn default() -> Self {
        Self::new()
    }
}
