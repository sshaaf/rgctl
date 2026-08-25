use serde::{Deserialize, Serialize};

use super::ShoppingCartItem;

/// CoolStore-shaped cart with mutable pricing totals (CPG field-write target).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingCart {
    pub cart_id: String,
    pub cart_item_total: f64,
    pub cart_item_promo_savings: f64,
    pub shipping_total: f64,
    pub shipping_promo_savings: f64,
    pub cart_total: f64,
    pub shopping_cart_item_list: Vec<ShoppingCartItem>,
}

impl ShoppingCart {
    pub fn new(cart_id: impl Into<String>) -> Self {
        Self {
            cart_id: cart_id.into(),
            cart_item_total: 0.0,
            cart_item_promo_savings: 0.0,
            shipping_total: 0.0,
            shipping_promo_savings: 0.0,
            cart_total: 0.0,
            shopping_cart_item_list: Vec::new(),
        }
    }

    pub fn reset_shopping_cart_item_list(&mut self) {
        self.shopping_cart_item_list = Vec::new();
    }

    pub fn add_shopping_cart_item(&mut self, sci: ShoppingCartItem) {
        self.shopping_cart_item_list.push(sci);
    }

    pub fn remove_shopping_cart_item(&mut self, index: usize) {
        if index < self.shopping_cart_item_list.len() {
            self.shopping_cart_item_list.remove(index);
        }
    }
}
