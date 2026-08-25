#ifndef EC_PRODUCT_SERVICE_H
#define EC_PRODUCT_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int product_get(sqlite3 *db, int id, void *out);
int product_list(sqlite3 *db, int *count);
int product_create(sqlite3 *db, const char *a, const char *b);
int product_to_dto(const void *entity, char *buf, size_t len);
#endif
