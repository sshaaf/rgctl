#ifndef EC_CART_REPO_H
#define EC_CART_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int cart_repo_find_by_user(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int cart_repo_add_item(sqlite3 *db, int user_id, int product_id, int qty);
int cart_repo_clear(sqlite3 *db, int user_id);
int cart_repo_count(sqlite3 *db, int *count);
#endif
