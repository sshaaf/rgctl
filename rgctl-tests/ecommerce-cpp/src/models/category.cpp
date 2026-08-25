#include "ecommerce/models/category.hpp"
namespace ecommerce::models {
void CategoryHelper::init(Category* obj) { if (obj) { obj->id = 0; } }
bool CategoryHelper::validate(const Category* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
