#include "ecommerce/repositories/product_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int product_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM products WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int product_repo_find_by_id(sqlite3 *db, int id, void *out) {
    if (!db || !out) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM products WHERE id=%d", id);
    return db_exec(db, sql);
}

int product_repo_find_by_category(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM products WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int product_repo_create(sqlite3 *db, const void *entity) {
    if (!db || !entity) return -1;
    return db_exec(db, "INSERT INTO products DEFAULT VALUES");
}

