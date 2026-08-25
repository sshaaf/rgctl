#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class CartItemHelper {
public:
    static void init(CartItem* obj);
    static bool validate(const CartItem* obj);
};
}  // namespace ecommerce::models
