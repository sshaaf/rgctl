#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class ProductHelper {
public:
    static void init(Product* obj);
    static bool validate(const Product* obj);
};
}  // namespace ecommerce::models
