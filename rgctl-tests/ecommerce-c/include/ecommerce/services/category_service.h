#ifndef EC_CATEGORY_SERVICE_H
#define EC_CATEGORY_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int category_get(sqlite3 *db, int id, void *out);
int category_list(sqlite3 *db, int *count);
int category_create(sqlite3 *db, const char *a, const char *b);
int category_to_dto(const void *entity, char *buf, size_t len);
#endif
