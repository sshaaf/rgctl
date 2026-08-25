#include "ecommerce/repositories/order_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int order_repo_find_by_id(sqlite3 *db, int id, void *out) {
    if (!db || !out) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM orders WHERE id=%d", id);
    return db_exec(db, sql);
}

int order_repo_find_by_user(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM orders WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int order_repo_create(sqlite3 *db, const void *entity) {
    if (!db || !entity) return -1;
    return db_exec(db, "INSERT INTO orders DEFAULT VALUES");
}

int order_repo_update_status(sqlite3 *db, int id, int value) {
    if (!db) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "UPDATE orders SET status=%d WHERE id=%d", value, id);
    return db_exec(db, sql);
}

