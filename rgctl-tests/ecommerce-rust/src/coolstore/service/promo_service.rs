use std::collections::HashMap;

use crate::coolstore::model::ShoppingCart;

pub struct PromoService {
    percent_off_by_item: HashMap<String, f64>,
}

impl PromoService {
    pub fn new() -> Self {
        let mut percent_off_by_item = HashMap::new();
        percent_off_by_item.insert("329299".to_string(), 0.25);
        Self { percent_off_by_item }
    }

    pub fn apply_cart_item_promotions(&self, shopping_cart: &mut ShoppingCart) {
        if shopping_cart.shopping_cart_item_list.is_empty() {
            return;
        }
        for sci in &mut shopping_cart.shopping_cart_item_list {
            let Some(ref product) = sci.product else {
                continue;
            };
            if let Some(&pct) = self.percent_off_by_item.get(&product.item_id) {
                sci.promo_savings = product.price * pct * -1.0;
                sci.price = product.price * (1.0 - pct);
            }
        }
    }

    pub fn apply_shipping_promotions(&self, shopping_cart: &mut ShoppingCart) {
        if shopping_cart.cart_item_total >= 75.0 {
            shopping_cart.shipping_promo_savings = shopping_cart.shipping_total * -1.0;
            shopping_cart.shipping_total = 0.0;
        }
    }
}

impl Default for PromoService {
    fn default() -> Self {
        Self::new()
    }
}
