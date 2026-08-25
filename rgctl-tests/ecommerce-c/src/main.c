#include "ecommerce/db/sqlite.h"
#include "ecommerce/handlers/auth_handler.h"
#include "ecommerce/handlers/cart_handler.h"
#include "ecommerce/handlers/order_handler.h"
#include "ecommerce/handlers/health_handler.h"
#include "ecommerce/coolstore/handlers/product_handler.h"
#include "ecommerce/coolstore/handlers/cart_handler.h"
#include "ecommerce/coolstore/handlers/order_handler.h"
#include <stdio.h>

int main(int argc, char **argv) {
    sqlite3 *db = NULL;
    if (db_open("ecommerce.db", &db) != 0) {
        fprintf(stderr, "failed to open database\n");
        return 1;
    }
    handle_health(db, NULL);
    if (argc > 1 && argv[1]) {
        handle_auth(db, argv[1]);
        handle_cart(db, argv[1]);
        handle_order(db, argv[1]);
        /* CoolStore dual-API (/services/products|cart|orders) */
        handle_coolstore_products(argv[1]);
        handle_coolstore_cart(argv[1]);
        handle_coolstore_orders(argv[1]);
    }
    db_close(db);
    return 0;
}
