#include "ecommerce/repositories/review_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int review_repo_find_by_product(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM reviews WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int review_repo_create(sqlite3 *db, const void *entity) {
    if (!db || !entity) return -1;
    return db_exec(db, "INSERT INTO reviews DEFAULT VALUES");
}

int review_repo_count(sqlite3 *db, int *count) {
    if (!db || !count) return -1;
    *count = 0;
    db_exec(db, "SELECT COUNT(*) FROM reviews");
    return 0;
}

