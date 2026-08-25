#pragma once
#include "ecommerce/coolstore/services/coolstore_order_service.hpp"
#include "ecommerce/coolstore/services/coolstore_product_service.hpp"
#include "ecommerce/coolstore/services/promo_service.hpp"
#include "ecommerce/coolstore/services/shipping_service.hpp"
#include "ecommerce/coolstore/services/shopping_cart_service.hpp"

namespace ecommerce::coolstore {

/** Shared in-memory CoolStore services (dual-API runtime). */
struct CoolstoreRuntime {
    CoolstoreProductService productService;
    PromoService promoService;
    ShippingService shippingService;
    CoolstoreOrderService orderService;
    ShoppingCartService shoppingCartService;

    CoolstoreRuntime()
        : shoppingCartService(productService, promoService, shippingService, orderService) {}
};

CoolstoreRuntime& runtime();

}  // namespace ecommerce::coolstore
