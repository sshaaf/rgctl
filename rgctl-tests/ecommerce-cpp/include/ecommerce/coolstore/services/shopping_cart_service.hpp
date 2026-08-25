#pragma once
#include "ecommerce/coolstore/models/catalog_product.hpp"
#include "ecommerce/coolstore/models/shopping_cart.hpp"
#include "ecommerce/coolstore/services/coolstore_order_service.hpp"
#include "ecommerce/coolstore/services/coolstore_product_service.hpp"
#include "ecommerce/coolstore/services/promo_service.hpp"
#include "ecommerce/coolstore/services/shipping_service.hpp"
#include <string>
#include <unordered_map>
#include <vector>

namespace ecommerce::coolstore {

class ShoppingCartService {
public:
    ShoppingCartService(CoolstoreProductService& productService, PromoService& promoService,
                        ShippingService& shippingService, CoolstoreOrderService& orderService);

    ShoppingCart& getShoppingCart(const std::string& cartId);
    CatalogProduct* getProduct(const std::string& itemId);
    ShoppingCart& checkOutShoppingCart(const std::string& cartId);
    /** Mutates ShoppingCart totals — primary CPG field-write site. */
    void priceShoppingCart(ShoppingCart* sc);
    std::vector<ShoppingCartItem> dedupeCartItems(const std::vector<ShoppingCartItem>& cartItems);

private:
    void initShoppingCartForPricing(ShoppingCart& sc);

    CoolstoreProductService& productService_;
    PromoService& promoService_;
    ShippingService& shippingService_;
    CoolstoreOrderService& orderService_;
    std::unordered_map<std::string, ShoppingCart> carts_;
};

}  // namespace ecommerce::coolstore
