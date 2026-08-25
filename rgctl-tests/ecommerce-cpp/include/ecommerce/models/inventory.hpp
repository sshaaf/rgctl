#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class InventoryHelper {
public:
    static void init(Inventory* obj);
    static bool validate(const Inventory* obj);
};
}  // namespace ecommerce::models
