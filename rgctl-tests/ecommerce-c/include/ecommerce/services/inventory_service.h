#ifndef EC_INVENTORY_SERVICE_H
#define EC_INVENTORY_SERVICE_H
#include <sqlite3.h>
#include "ecommerce/types.h"
int inventory_get_by_product(sqlite3 *db, int id, void *out);
int inventory_list(sqlite3 *db, int *count);
int inventory_adjust(sqlite3 *db, int product_id, int delta);
int inventory_to_dto(const void *entity, char *buf, size_t len);
#endif
