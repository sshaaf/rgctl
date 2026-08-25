use crate::coolstore::model::ShoppingCart;

pub struct ShippingService;

impl ShippingService {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_shipping(&self, sc: &ShoppingCart) -> f64 {
        let total = sc.cart_item_total;
        if (0.0..25.0).contains(&total) {
            return 2.99;
        }
        if (25.0..50.0).contains(&total) {
            return 4.99;
        }
        if (50.0..75.0).contains(&total) {
            return 6.99;
        }
        if (75.0..100.0).contains(&total) {
            return 8.99;
        }
        if total >= 100.0 {
            return 10.99;
        }
        0.0
    }

    pub fn calculate_shipping_insurance(&self, sc: &ShoppingCart) -> f64 {
        let total = sc.cart_item_total;
        if (25.0..100.0).contains(&total) {
            return round2(total * 0.02);
        }
        if total >= 100.0 {
            return round2(total * 0.015);
        }
        0.0
    }
}

impl Default for ShippingService {
    fn default() -> Self {
        Self::new()
    }
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
