#include "ecommerce/services/product_service.h"
#include "ecommerce/repositories/product_repository.h"
#include <stdio.h>
#include <string.h>

int product_get(sqlite3 *db, int id, void *out) { return product_repo_find_by_id(db, id, out); }

int product_list(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int product_create(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int product_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

