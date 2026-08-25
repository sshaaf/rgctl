#include "ecommerce/services/review_service.h"
#include "ecommerce/repositories/review_repository.h"
#include <stdio.h>
#include <string.h>

int review_create(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int review_list_for_product(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int review_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

