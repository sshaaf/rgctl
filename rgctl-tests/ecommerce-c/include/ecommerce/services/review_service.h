#ifndef EC_REVIEW_SERVICE_H
#define EC_REVIEW_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int review_create(sqlite3 *db, const char *a, const char *b);
int review_list_for_product(sqlite3 *db, int *count);
int review_to_dto(const void *entity, char *buf, size_t len);
#endif
