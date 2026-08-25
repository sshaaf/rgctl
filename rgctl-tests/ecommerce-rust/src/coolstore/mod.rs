//! CoolStore dual-API (`/services/products|cart|orders`) — in-memory catalog/carts/orders.
//! Mirrors the Java CoolStore fixture for cross-language CPG comparison.

pub mod model;
pub mod rest;
pub mod service;

use std::sync::Arc;

use service::{
    CoolstoreOrderService, CoolstoreProductService, PromoService, ShippingService,
    ShoppingCartService,
};

/// Shared CoolStore services (cloneable Axum state fragment).
#[derive(Clone)]
pub struct CoolstoreState {
    pub products: Arc<CoolstoreProductService>,
    pub carts: Arc<ShoppingCartService>,
    pub orders: Arc<CoolstoreOrderService>,
}

impl CoolstoreState {
    pub fn new() -> Self {
        let products = Arc::new(CoolstoreProductService::new());
        let promo = Arc::new(PromoService::new());
        let shipping = Arc::new(ShippingService::new());
        let orders = Arc::new(CoolstoreOrderService::new());
        let carts = Arc::new(ShoppingCartService::new(
            Arc::clone(&products),
            Arc::clone(&promo),
            Arc::clone(&shipping),
            Arc::clone(&orders),
        ));
        Self {
            products,
            carts,
            orders,
        }
    }
}

impl Default for CoolstoreState {
    fn default() -> Self {
        Self::new()
    }
}
