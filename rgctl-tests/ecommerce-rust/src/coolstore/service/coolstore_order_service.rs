use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::RwLock;

use crate::coolstore::model::{CoolstoreOrder, CoolstoreOrderItem, ShoppingCart};

pub struct CoolstoreOrderService {
    seq: AtomicI64,
    orders: RwLock<HashMap<i64, CoolstoreOrder>>,
}

impl CoolstoreOrderService {
    pub fn new() -> Self {
        Self {
            seq: AtomicI64::new(1),
            orders: RwLock::new(HashMap::new()),
        }
    }

    pub fn process(&self, cart: &ShoppingCart) -> CoolstoreOrder {
        let mut order = CoolstoreOrder::new();
        order.order_id = self.seq.fetch_add(1, Ordering::SeqCst);
        order.cart_id = cart.cart_id.clone();
        order.cart_total = cart.cart_total;
        let mut items = Vec::new();
        for sci in &cart.shopping_cart_item_list {
            if let Some(ref product) = sci.product {
                items.push(CoolstoreOrderItem::new(
                    product.item_id.clone(),
                    sci.quantity,
                    sci.price,
                ));
            }
        }
        order.items = items;
        self.orders
            .write()
            .expect("orders lock")
            .insert(order.order_id, order.clone());
        order
    }

    pub fn get_orders(&self) -> Vec<CoolstoreOrder> {
        self.orders
            .read()
            .expect("orders lock")
            .values()
            .cloned()
            .collect()
    }

    pub fn get_order_by_id(&self, order_id: i64) -> Option<CoolstoreOrder> {
        self.orders.read().expect("orders lock").get(&order_id).cloned()
    }
}

impl Default for CoolstoreOrderService {
    fn default() -> Self {
        Self::new()
    }
}
