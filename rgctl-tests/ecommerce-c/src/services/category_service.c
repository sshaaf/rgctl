#include "ecommerce/services/category_service.h"
#include "ecommerce/repositories/category_repository.h"
#include <stdio.h>
#include <string.h>

int category_get(sqlite3 *db, int id, void *out) { (void)id; (void)out; return 0; }

int category_list(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int category_create(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int category_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

