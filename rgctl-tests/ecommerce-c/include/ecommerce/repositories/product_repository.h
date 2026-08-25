#ifndef EC_PRODUCT_REPO_H
#define EC_PRODUCT_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int product_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int product_repo_find_by_id(sqlite3 *db, int id, void *out);
int product_repo_find_by_category(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int product_repo_create(sqlite3 *db, const void *entity);
#endif
