#include "ecommerce/services/order_service.hpp"
#include "ecommerce/services/cart_service.hpp"
#include "ecommerce/services/product_service.hpp"
#include "ecommerce/services/inventory_service.hpp"
#include "ecommerce/repositories/order_repository.hpp"
#include "ecommerce/types.hpp"

namespace ecommerce::services {

int checkout(sqlite3* db, int user_id) {
    if (!db || user_id <= 0) return -1;
    CartItem items[32];
    int count = 0;
    if (get_user_cart(db, user_id, items, sizeof(items), &count) != 0) return -1;
    if (count == 0) return -2;
    double total = 0;
    for (int i = 0; i < count; i++) {
        Product product{};
        if (get_product(db, items[i].product_id, &product) != 0) continue;
        Inventory inv{};
        if (get_by_product(db, items[i].product_id, &inv) != 0) return -3;
        if (inv.quantity < items[i].quantity) return -4;
        total += product.price * items[i].quantity;
    }
    Order order{};
    order.user_id = user_id;
    order.total = total;
    if (repositories::order_create(db, &order) != 0) return -5;
    clear_cart(db, user_id);
    return order.id;
}

int get_order(sqlite3* db, int id, void* out) {
    return repositories::order_find_by_id(db, id, out);
}

}  // namespace ecommerce::services
