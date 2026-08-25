#ifndef EC_ORDER_REPO_H
#define EC_ORDER_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int order_repo_find_by_id(sqlite3 *db, int id, void *out);
int order_repo_find_by_user(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int order_repo_create(sqlite3 *db, const void *entity);
int order_repo_update_status(sqlite3 *db, int id, int value);
#endif
