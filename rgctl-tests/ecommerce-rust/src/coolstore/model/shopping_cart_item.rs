use serde::{Deserialize, Serialize};

use super::CatalogProduct;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingCartItem {
    pub price: f64,
    pub quantity: i32,
    pub promo_savings: f64,
    pub product: Option<CatalogProduct>,
}

impl ShoppingCartItem {
    pub fn new() -> Self {
        Self {
            price: 0.0,
            quantity: 0,
            promo_savings: 0.0,
            product: None,
        }
    }
}

impl Default for ShoppingCartItem {
    fn default() -> Self {
        Self::new()
    }
}
