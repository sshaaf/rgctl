#ifndef EC_INVENTORY_REPO_H
#define EC_INVENTORY_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int inventory_repo_find_by_product(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int inventory_repo_update_qty(sqlite3 *db, int id, int value);
int inventory_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
#endif
