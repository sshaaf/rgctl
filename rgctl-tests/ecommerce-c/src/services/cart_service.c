#include "ecommerce/services/cart_service.h"
#include "ecommerce/repositories/cart_repository.h"
#include "ecommerce/repositories/product_repository.h"
#include "ecommerce/services/product_service.h"
#include "ecommerce/types.h"
#include <stdio.h>
#include <string.h>

int cart_get_user_cart(sqlite3 *db, int user_id, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    return cart_repo_find_by_user(db, user_id, out, out_cap, count);
}

int cart_add_item(sqlite3 *db, int user_id, int product_id, int qty) {
    if (!db || qty <= 0) return -1;
    product_t p;
    if (product_get(db, product_id, &p) != 0) return -2;
    return cart_repo_add_item(db, user_id, product_id, qty);
}

int cart_clear(sqlite3 *db, int user_id) { return cart_repo_clear(db, user_id); }

int cart_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

