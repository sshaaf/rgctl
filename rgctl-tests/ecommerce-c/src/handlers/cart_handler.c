#include "ecommerce/handlers/cart_handler.h"
#include "ecommerce/services/cart_service.h"
#include <stdlib.h>
#include <string.h>

int handle_cart(sqlite3 *db, const char *query) {
    if (!db || !query) return -1;
    int user_id = 1;
    if (strstr(query, "add") != NULL) {
        return cart_add_item(db, user_id, 1, 1);
    }
    cart_item_t items[16];
    int count = 0;
    return cart_get_user_cart(db, user_id, items, sizeof(items), &count);
}

