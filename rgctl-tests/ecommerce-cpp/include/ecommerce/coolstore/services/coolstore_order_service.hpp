#pragma once
#include "ecommerce/coolstore/models/coolstore_order.hpp"
#include "ecommerce/coolstore/models/shopping_cart.hpp"
#include <optional>
#include <unordered_map>
#include <vector>

namespace ecommerce::coolstore {

class CoolstoreOrderService {
public:
    CoolstoreOrder process(ShoppingCart& cart);
    std::vector<CoolstoreOrder> getOrders() const;
    std::optional<CoolstoreOrder> getOrderById(long orderId) const;

private:
    long seq_{1};
    std::unordered_map<long, CoolstoreOrder> orders_;
};

}  // namespace ecommerce::coolstore
