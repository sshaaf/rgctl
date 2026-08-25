#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class OrderHelper {
public:
    static void init(Order* obj);
    static bool validate(const Order* obj);
};
}  // namespace ecommerce::models
