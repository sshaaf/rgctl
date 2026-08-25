#include "ecommerce/services/inventory_service.h"
#include "ecommerce/repositories/inventory_repository.h"
#include "ecommerce/models/inventory.h"
#include <stdio.h>
#include <string.h>

int inventory_get_by_product(sqlite3 *db, int product_id, void *out) { return inventory_repo_find_by_product(db, product_id, out, sizeof(inventory_t), NULL); }

int inventory_list(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int inventory_adjust(sqlite3 *db, int *count) { if (count) *count = 0; return 0; }

int inventory_to_dto(const void *entity, char *buf, size_t len) {
    if (!entity || !buf || len == 0) return -1;
    snprintf(buf, len, "{}");
    return 0;
}

