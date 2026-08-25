#pragma once
#include "ecommerce/coolstore/models/shopping_cart.hpp"
#include <string>
#include <unordered_map>

namespace ecommerce::coolstore {

class PromoService {
public:
    PromoService();
    void applyCartItemPromotions(ShoppingCart& shoppingCart);
    void applyShippingPromotions(ShoppingCart& shoppingCart);

private:
    std::unordered_map<std::string, double> percentOffByItem_;
};

}  // namespace ecommerce::coolstore
