#include "ecommerce/repositories/category_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int category_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM categorys WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int category_repo_find_by_id(sqlite3 *db, int id, void *out) {
    if (!db || !out) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM categorys WHERE id=%d", id);
    return db_exec(db, sql);
}

int category_repo_create(sqlite3 *db, const void *entity) {
    if (!db || !entity) return -1;
    return db_exec(db, "INSERT INTO categorys DEFAULT VALUES");
}

int category_repo_count(sqlite3 *db, int *count) {
    if (!db || !count) return -1;
    *count = 0;
    db_exec(db, "SELECT COUNT(*) FROM categorys");
    return 0;
}

