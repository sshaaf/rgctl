#pragma once
#include "ecommerce/coolstore/models/coolstore_order_item.hpp"
#include <string>
#include <vector>

namespace ecommerce::coolstore {

class CoolstoreOrder {
public:
    long orderId{};
    std::string cartId;
    double cartTotal{};
    std::vector<CoolstoreOrderItem> items;
};

}  // namespace ecommerce::coolstore
