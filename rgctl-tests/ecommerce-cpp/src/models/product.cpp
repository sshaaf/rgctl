#include "ecommerce/models/product.hpp"
namespace ecommerce::models {
void ProductHelper::init(Product* obj) { if (obj) { obj->id = 0; } }
bool ProductHelper::validate(const Product* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
