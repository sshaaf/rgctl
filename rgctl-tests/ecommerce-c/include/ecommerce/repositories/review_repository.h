#ifndef EC_REVIEW_REPO_H
#define EC_REVIEW_REPO_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int review_repo_find_by_product(sqlite3 *db, int key, void *out, size_t out_cap, int *count);
int review_repo_create(sqlite3 *db, const void *entity);
int review_repo_count(sqlite3 *db, int *count);
#endif
