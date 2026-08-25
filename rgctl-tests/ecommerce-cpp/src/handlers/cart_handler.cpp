#include "ecommerce/handlers/cart_handler.hpp"
#include "ecommerce/services/order_service.hpp"
#include "ecommerce/services/cart_service.hpp"
#include "ecommerce/types.hpp"
#include <cstring>

namespace ecommerce::handlers {
int handle_cart(sqlite3* db, const char* query) {
    if (!db) return -1;
    CartItem items[16];
    int count = 0;
    return services::get_user_cart(db, 1, items, sizeof(items), &count);
}
}  // namespace ecommerce::handlers
