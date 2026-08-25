#include "ecommerce/handlers/order_handler.hpp"
#include "ecommerce/services/order_service.hpp"
#include "ecommerce/services/cart_service.hpp"
#include "ecommerce/types.hpp"
#include <cstring>

namespace ecommerce::handlers {
int handle_order(sqlite3* db, const char* query) {
    if (!db || !query) return -1;
    if (std::strstr(query, "checkout") != nullptr) {
        return services::checkout(db, 1);
    }
    return 0;
}
}  // namespace ecommerce::handlers
