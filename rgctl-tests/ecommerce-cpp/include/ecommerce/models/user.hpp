#pragma once
#include "ecommerce/types.hpp"
namespace ecommerce::models {
class UserHelper {
public:
    static void init(User* obj);
    static bool validate(const User* obj);
};
}  // namespace ecommerce::models
