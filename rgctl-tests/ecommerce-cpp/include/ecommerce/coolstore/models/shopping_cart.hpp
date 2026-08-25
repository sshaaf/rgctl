#pragma once
#include "ecommerce/coolstore/models/shopping_cart_item.hpp"
#include <string>
#include <vector>

namespace ecommerce::coolstore {

/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */
class ShoppingCart {
public:
    ShoppingCart() = default;
    explicit ShoppingCart(std::string cartId) : cartId(std::move(cartId)) {}

    std::string cartId;
    double cartItemTotal{};
    double cartItemPromoSavings{};
    double shippingTotal{};
    double shippingPromoSavings{};
    double cartTotal{};
    std::vector<ShoppingCartItem> shoppingCartItemList;

    void resetShoppingCartItemList() { shoppingCartItemList.clear(); }

    void addShoppingCartItem(const ShoppingCartItem& sci) {
        shoppingCartItemList.push_back(sci);
    }

    bool removeShoppingCartItem(const ShoppingCartItem* sci) {
        if (!sci) return false;
        for (auto it = shoppingCartItemList.begin(); it != shoppingCartItemList.end(); ++it) {
            if (&(*it) == sci) {
                shoppingCartItemList.erase(it);
                return true;
            }
        }
        return false;
    }
};

}  // namespace ecommerce::coolstore
