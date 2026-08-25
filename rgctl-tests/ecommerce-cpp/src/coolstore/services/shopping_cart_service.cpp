#include "ecommerce/coolstore/services/shopping_cart_service.hpp"

namespace ecommerce::coolstore {

ShoppingCartService::ShoppingCartService(CoolstoreProductService& productService,
                                         PromoService& promoService,
                                         ShippingService& shippingService,
                                         CoolstoreOrderService& orderService)
    : productService_(productService),
      promoService_(promoService),
      shippingService_(shippingService),
      orderService_(orderService) {}

ShoppingCart& ShoppingCartService::getShoppingCart(const std::string& cartId) {
    auto it = carts_.find(cartId);
    if (it == carts_.end()) {
        it = carts_.emplace(cartId, ShoppingCart(cartId)).first;
    }
    return it->second;
}

CatalogProduct* ShoppingCartService::getProduct(const std::string& itemId) {
    return productService_.getProductByItemId(itemId);
}

ShoppingCart& ShoppingCartService::checkOutShoppingCart(const std::string& cartId) {
    ShoppingCart& cart = getShoppingCart(cartId);
    priceShoppingCart(&cart);
    orderService_.process(cart);
    cart.resetShoppingCartItemList();
    priceShoppingCart(&cart);
    return cart;
}

void ShoppingCartService::initShoppingCartForPricing(ShoppingCart& sc) {
    sc.cartItemTotal = 0;
    sc.cartItemPromoSavings = 0;
    sc.shippingTotal = 0;
    sc.shippingPromoSavings = 0;
    sc.cartTotal = 0;

    for (auto& sci : sc.shoppingCartItemList) {
        if (sci.hasProduct) {
            CatalogProduct* p = getProduct(sci.product.itemId);
            if (p) {
                sci.product = *p;
                sci.price = p->price;
            }
        }
        sci.promoSavings = 0;
    }
}

/** Mutates ShoppingCart totals — primary CPG field-write site. */
void ShoppingCartService::priceShoppingCart(ShoppingCart* sc) {
    if (!sc) return;
    initShoppingCartForPricing(*sc);

    if (!sc->shoppingCartItemList.empty()) {
        promoService_.applyCartItemPromotions(*sc);

        for (auto& sci : sc->shoppingCartItemList) {
            sc->cartItemPromoSavings =
                sc->cartItemPromoSavings + sci.promoSavings * sci.quantity;
            sc->cartItemTotal = sc->cartItemTotal + sci.price * sci.quantity;
        }

        sc->shippingTotal = shippingService_.calculateShipping(*sc);
        if (sc->cartItemTotal >= 25) {
            sc->shippingTotal =
                sc->shippingTotal + shippingService_.calculateShippingInsurance(*sc);
        }
    }

    promoService_.applyShippingPromotions(*sc);
    sc->cartTotal = sc->cartItemTotal + sc->shippingTotal;
}

std::vector<ShoppingCartItem> ShoppingCartService::dedupeCartItems(
    const std::vector<ShoppingCartItem>& cartItems) {
    std::unordered_map<std::string, int> quantityMap;
    for (const auto& sci : cartItems) {
        if (!sci.hasProduct) continue;
        quantityMap[sci.product.itemId] += sci.quantity;
    }
    std::vector<ShoppingCartItem> result;
    for (const auto& [itemId, quantity] : quantityMap) {
        CatalogProduct* p = getProduct(itemId);
        if (!p) continue;
        ShoppingCartItem newItem;
        newItem.quantity = quantity;
        newItem.price = p->price;
        newItem.product = *p;
        newItem.hasProduct = true;
        result.push_back(newItem);
    }
    return result;
}

}  // namespace ecommerce::coolstore
