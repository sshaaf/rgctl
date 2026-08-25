#pragma once
#include <string>

namespace ecommerce {

struct User {
    int id{};
    std::string email;
    std::string password_hash;
};

struct Product {
    int id{};
    std::string name;
    double price{};
    int category_id{};
};

struct CartItem {
    int id{};
    int user_id{};
    int product_id{};
    int quantity{};
};

struct Order {
    int id{};
    int user_id{};
    double total{};
    int status{};
};

struct Inventory {
    int id{};
    int product_id{};
    int quantity{};
};

struct Category {
    int id{};
    std::string name;
};

}  // namespace ecommerce
