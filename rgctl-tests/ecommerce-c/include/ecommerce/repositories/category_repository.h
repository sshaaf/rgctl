#ifndef EC_CATEGORY_REPO_H
#define EC_CATEGORY_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int category_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int category_repo_find_by_id(sqlite3 *db, int id, void *out);
int category_repo_create(sqlite3 *db, const void *entity);
int category_repo_count(sqlite3 *db, int *count);
#endif
