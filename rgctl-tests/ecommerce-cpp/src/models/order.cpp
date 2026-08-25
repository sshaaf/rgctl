#include "ecommerce/models/order.hpp"
namespace ecommerce::models {
void OrderHelper::init(Order* obj) { if (obj) { obj->id = 0; } }
bool OrderHelper::validate(const Order* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
