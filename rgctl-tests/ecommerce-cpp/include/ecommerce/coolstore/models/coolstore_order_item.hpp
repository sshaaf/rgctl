#pragma once
#include <string>

namespace ecommerce::coolstore {

class CoolstoreOrderItem {
public:
    CoolstoreOrderItem() = default;
    CoolstoreOrderItem(std::string productId, int quantity, double price)
        : productId(std::move(productId)), quantity(quantity), price(price) {}

    std::string productId;
    int quantity{};
    double price{};
};

}  // namespace ecommerce::coolstore
