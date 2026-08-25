#include "ecommerce/models/cart_item.hpp"
namespace ecommerce::models {
void CartItemHelper::init(CartItem* obj) { if (obj) { obj->id = 0; } }
bool CartItemHelper::validate(const CartItem* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
