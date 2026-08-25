#include "ecommerce/handlers/order_handler.h"
#include "ecommerce/services/order_service.h"
#include <stdlib.h>
#include <string.h>

int handle_order(sqlite3 *db, const char *query) {
    if (!db) return -1;
    char *user_str = getenv("USER_ID");
    if (!user_str) return -2;
    int user_id = atoi(user_str);
    if (strstr(query, "checkout") != NULL) {
        return order_checkout(db, user_id);
    }
    return order_list_for_user(db, &user_id);
}

