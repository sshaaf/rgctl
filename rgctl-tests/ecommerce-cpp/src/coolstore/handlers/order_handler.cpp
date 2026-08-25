#include "ecommerce/coolstore/handlers/order_handler.hpp"
#include "ecommerce/coolstore/runtime.hpp"
#include <cstdio>
#include <cstring>

namespace ecommerce::coolstore::handlers {
namespace {

const char* skip_method(const char* query) {
    if (!query) return nullptr;
    if (std::strncmp(query, "GET ", 4) == 0) return query + 4;
    if (std::strncmp(query, "POST ", 5) == 0) return query + 5;
    if (std::strncmp(query, "DELETE ", 7) == 0) return query + 7;
    return query;
}

}  // namespace

int handle_orders(const char* query) {
    if (!query) return -1;
    const char* path = skip_method(query);
    if (!path || std::strstr(path, "/services/orders") == nullptr) return 0;

    auto& orders = runtime().orderService;
    long orderId = 0;
    if (std::sscanf(path, "/services/orders/%ld", &orderId) == 1) {
        return orders.getOrderById(orderId) ? 0 : -1;
    }
    (void)orders.getOrders();
    return 0;
}

}  // namespace ecommerce::coolstore::handlers
