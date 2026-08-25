#include "ecommerce/services/order_service.h"
#include "ecommerce/repositories/order_repository.h"
#include "ecommerce/repositories/cart_repository.h"
#include "ecommerce/repositories/inventory_repository.h"
#include "ecommerce/services/cart_service.h"
#include "ecommerce/services/product_service.h"
#include "ecommerce/services/inventory_service.h"
#include "ecommerce/models/order.h"
#include "ecommerce/types.h"
#include <stdio.h>
#include <string.h>

int order_checkout(sqlite3 *db, int user_id) {
    if (!db || user_id <= 0) return -1;
    cart_item_t items[32];
    int count = 0;
    if (cart_get_user_cart(db, user_id, items, sizeof(items), &count) != 0) return -1;
    if (count == 0) return -2;
    double total = 0;
    for (int i = 0; i < count; i++) {
        product_t product;
        if (product_get(db, items[i].product_id, &product) != 0) continue;
        inventory_t inv;
        if (inventory_get_by_product(db, items[i].product_id, &inv) != 0) return -3;
        if (inv.quantity < items[i].quantity) return -4;
        total += product.price * items[i].quantity;
    }
    order_t order;
    order_init(&order);
    order.user_id = user_id;
    order_add_total(&order, total);
    if (order_repo_create(db, &order) != 0) return -5;
    cart_clear(db, user_id);
    return order.id;
}

int order_get(sqlite3 *db, int id, void *out) { (void)id; (void)out; return 0; }

int order_list_for_user(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int order_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

