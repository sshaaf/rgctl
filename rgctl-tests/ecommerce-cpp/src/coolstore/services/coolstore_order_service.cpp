#include "ecommerce/coolstore/services/coolstore_order_service.hpp"

namespace ecommerce::coolstore {

CoolstoreOrder CoolstoreOrderService::process(ShoppingCart& cart) {
    CoolstoreOrder order;
    order.orderId = seq_++;
    order.cartId = cart.cartId;
    order.cartTotal = cart.cartTotal;
    for (const auto& sci : cart.shoppingCartItemList) {
        if (!sci.hasProduct) continue;
        order.items.emplace_back(sci.product.itemId, sci.quantity, sci.price);
    }
    orders_[order.orderId] = order;
    return order;
}

std::vector<CoolstoreOrder> CoolstoreOrderService::getOrders() const {
    std::vector<CoolstoreOrder> out;
    out.reserve(orders_.size());
    for (const auto& [_, o] : orders_) {
        out.push_back(o);
    }
    return out;
}

std::optional<CoolstoreOrder> CoolstoreOrderService::getOrderById(long orderId) const {
    auto it = orders_.find(orderId);
    if (it == orders_.end()) return std::nullopt;
    return it->second;
}

}  // namespace ecommerce::coolstore
