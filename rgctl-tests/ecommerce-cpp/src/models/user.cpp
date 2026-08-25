#include "ecommerce/models/user.hpp"
namespace ecommerce::models {
void UserHelper::init(User* obj) { if (obj) { obj->id = 0; } }
bool UserHelper::validate(const User* obj) { return obj != nullptr; }
}  // namespace ecommerce::models
