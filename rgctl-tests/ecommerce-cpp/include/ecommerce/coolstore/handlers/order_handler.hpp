#pragma once

namespace ecommerce::coolstore::handlers {

/** Dispatch GET /services/orders and GET /services/orders/{orderId}. */
int handle_orders(const char* query);

}  // namespace ecommerce::coolstore::handlers
