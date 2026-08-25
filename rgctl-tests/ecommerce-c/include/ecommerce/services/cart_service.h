#ifndef EC_CART_SERVICE_H
#define EC_CART_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int cart_get_user_cart(sqlite3 *db, int id, void *out);
int cart_add_item(sqlite3 *db, int user_id, int product_id, int qty);
int cart_clear(sqlite3 *db, int id, void *out);
int cart_to_dto(const void *entity, char *buf, size_t len);
#endif
