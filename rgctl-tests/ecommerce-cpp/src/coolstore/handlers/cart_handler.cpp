#include "ecommerce/coolstore/handlers/cart_handler.hpp"
#include "ecommerce/coolstore/runtime.hpp"
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

namespace ecommerce::coolstore::handlers {
namespace {

const char* skip_method(const char* query) {
    if (!query) return nullptr;
    if (std::strncmp(query, "GET ", 4) == 0) return query + 4;
    if (std::strncmp(query, "POST ", 5) == 0) return query + 5;
    if (std::strncmp(query, "DELETE ", 7) == 0) return query + 7;
    return query;
}

bool is_delete(const char* query) {
    return query && (std::strncmp(query, "DELETE ", 7) == 0);
}

ShoppingCart& cart_add(ShoppingCartService& svc, const std::string& cartId,
                       const std::string& itemId, int quantity) {
    ShoppingCart& cart = svc.getShoppingCart(cartId);
    CatalogProduct* product = svc.getProduct(itemId);
    if (!product) return cart;

    ShoppingCartItem sci;
    sci.product = *product;
    sci.hasProduct = true;
    sci.quantity = quantity;
    sci.price = product->price;
    cart.addShoppingCartItem(sci);
    svc.priceShoppingCart(&cart);
    cart.shoppingCartItemList = svc.dedupeCartItems(cart.shoppingCartItemList);
    svc.priceShoppingCart(&cart);
    return cart;
}

ShoppingCart& cart_delete(ShoppingCartService& svc, const std::string& cartId,
                          const std::string& itemId, int quantity) {
    ShoppingCart& cart = svc.getShoppingCart(cartId);
    std::vector<ShoppingCartItem> kept;
    for (auto& sci : cart.shoppingCartItemList) {
        if (!sci.hasProduct || sci.product.itemId != itemId) {
            kept.push_back(sci);
            continue;
        }
        if (quantity >= sci.quantity) {
            continue;
        }
        sci.quantity -= quantity;
        kept.push_back(sci);
    }
    cart.shoppingCartItemList = std::move(kept);
    svc.priceShoppingCart(&cart);
    return cart;
}

}  // namespace

int handle_cart(const char* query) {
    if (!query) return -1;
    const char* path = skip_method(query);
    if (!path || std::strstr(path, "/services/cart") == nullptr) return 0;

    auto& svc = runtime().shoppingCartService;

    if (const char* checkout = std::strstr(path, "/services/cart/checkout/")) {
        std::string cartId = checkout + std::strlen("/services/cart/checkout/");
        svc.checkOutShoppingCart(cartId);
        return 0;
    }

    char cartId[64] = {};
    char itemId[32] = {};
    int quantity = 0;
    if (std::sscanf(path, "/services/cart/%63[^/]/%31[^/]/%d", cartId, itemId, &quantity) == 3) {
        if (is_delete(query)) {
            cart_delete(svc, cartId, itemId, quantity);
            return 0;
        }
        cart_add(svc, cartId, itemId, quantity);
        return 0;
    }

    if (std::sscanf(path, "/services/cart/%63s", cartId) == 1) {
        svc.getShoppingCart(cartId);
        return 0;
    }
    return 0;
}

}  // namespace ecommerce::coolstore::handlers
