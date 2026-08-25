use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::coolstore::model::{CatalogProduct, ShoppingCart, ShoppingCartItem};

use super::{CoolstoreOrderService, CoolstoreProductService, PromoService, ShippingService};

pub struct ShoppingCartService {
    product_service: Arc<CoolstoreProductService>,
    promo_service: Arc<PromoService>,
    shipping_service: Arc<ShippingService>,
    order_service: Arc<CoolstoreOrderService>,
    carts: RwLock<HashMap<String, ShoppingCart>>,
}

impl ShoppingCartService {
    pub fn new(
        product_service: Arc<CoolstoreProductService>,
        promo_service: Arc<PromoService>,
        shipping_service: Arc<ShippingService>,
        order_service: Arc<CoolstoreOrderService>,
    ) -> Self {
        Self {
            product_service,
            promo_service,
            shipping_service,
            order_service,
            carts: RwLock::new(HashMap::new()),
        }
    }

    pub fn get_shopping_cart(&self, cart_id: &str) -> ShoppingCart {
        let mut carts = self.carts.write().expect("carts lock");
        carts
            .entry(cart_id.to_string())
            .or_insert_with(|| ShoppingCart::new(cart_id))
            .clone()
    }

    fn put_cart(&self, cart: ShoppingCart) {
        self.carts
            .write()
            .expect("carts lock")
            .insert(cart.cart_id.clone(), cart);
    }

    pub fn get_product(&self, item_id: &str) -> Option<CatalogProduct> {
        self.product_service.get_product_by_item_id(item_id)
    }

    pub fn check_out_shopping_cart(&self, cart_id: &str) -> ShoppingCart {
        let mut cart = self.get_shopping_cart(cart_id);
        self.price_shopping_cart(&mut cart);
        self.order_service.process(&cart);
        cart.reset_shopping_cart_item_list();
        self.price_shopping_cart(&mut cart);
        self.put_cart(cart.clone());
        cart
    }

    /// Mutates ShoppingCart totals — primary CPG field-write site.
    pub fn price_shopping_cart(&self, sc: &mut ShoppingCart) {
        self.init_shopping_cart_for_pricing(sc);

        if !sc.shopping_cart_item_list.is_empty() {
            self.promo_service.apply_cart_item_promotions(sc);

            for sci in &sc.shopping_cart_item_list {
                sc.cart_item_promo_savings += sci.promo_savings * f64::from(sci.quantity);
                sc.cart_item_total += sci.price * f64::from(sci.quantity);
            }

            sc.shipping_total = self.shipping_service.calculate_shipping(sc);
            if sc.cart_item_total >= 25.0 {
                sc.shipping_total += self.shipping_service.calculate_shipping_insurance(sc);
            }
        }

        self.promo_service.apply_shipping_promotions(sc);
        sc.cart_total = sc.cart_item_total + sc.shipping_total;
    }

    fn init_shopping_cart_for_pricing(&self, sc: &mut ShoppingCart) {
        sc.cart_item_total = 0.0;
        sc.cart_item_promo_savings = 0.0;
        sc.shipping_total = 0.0;
        sc.shipping_promo_savings = 0.0;
        sc.cart_total = 0.0;

        for sci in &mut sc.shopping_cart_item_list {
            if let Some(ref product) = sci.product {
                if let Some(p) = self.get_product(&product.item_id) {
                    sci.price = p.price;
                    sci.product = Some(p);
                }
            }
            sci.promo_savings = 0.0;
        }
    }

    pub fn dedupe_cart_items(&self, cart_items: &[ShoppingCartItem]) -> Vec<ShoppingCartItem> {
        let mut quantity_map: HashMap<String, i32> = HashMap::new();
        for sci in cart_items {
            if let Some(ref product) = sci.product {
                *quantity_map.entry(product.item_id.clone()).or_insert(0) += sci.quantity;
            }
        }
        let mut result = Vec::new();
        for (item_id, quantity) in quantity_map {
            let Some(p) = self.get_product(&item_id) else {
                continue;
            };
            let mut new_item = ShoppingCartItem::new();
            new_item.quantity = quantity;
            new_item.price = p.price;
            new_item.product = Some(p);
            result.push(new_item);
        }
        result
    }

    pub fn add_item(&self, cart_id: &str, item_id: &str, quantity: i32) -> ShoppingCart {
        let mut cart = self.get_shopping_cart(cart_id);
        let Some(product) = self.get_product(item_id) else {
            return cart;
        };
        let mut sci = ShoppingCartItem::new();
        sci.product = Some(product.clone());
        sci.quantity = quantity;
        sci.price = product.price;
        cart.add_shopping_cart_item(sci);
        self.price_shopping_cart(&mut cart);
        cart.shopping_cart_item_list = self.dedupe_cart_items(&cart.shopping_cart_item_list);
        self.price_shopping_cart(&mut cart);
        self.put_cart(cart.clone());
        cart
    }

    pub fn delete_item(&self, cart_id: &str, item_id: &str, quantity: i32) -> ShoppingCart {
        let mut cart = self.get_shopping_cart(cart_id);
        let mut to_remove = Vec::new();
        for (idx, sci) in cart.shopping_cart_item_list.iter_mut().enumerate() {
            if sci.product.as_ref().is_some_and(|p| p.item_id == item_id) {
                if quantity >= sci.quantity {
                    to_remove.push(idx);
                } else {
                    sci.quantity -= quantity;
                }
            }
        }
        for idx in to_remove.into_iter().rev() {
            cart.remove_shopping_cart_item(idx);
        }
        self.price_shopping_cart(&mut cart);
        self.put_cart(cart.clone());
        cart
    }
}
