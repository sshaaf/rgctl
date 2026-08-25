#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class CategoryHelper {
public:
    static void init(Category* obj);
    static bool validate(const Category* obj);
};
}  // namespace ecommerce::models
