#include "ecommerce/models/inventory.hpp"
namespace ecommerce::models {
void InventoryHelper::init(Inventory* obj) { if (obj) { obj->id = 0; } }
bool InventoryHelper::validate(const Inventory* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
