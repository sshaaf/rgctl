#include "ecommerce/repositories/inventory_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int inventory_repo_find_by_product(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM inventorys WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int inventory_repo_update_qty(sqlite3 *db, int id, int value) {
    if (!db) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "UPDATE inventorys SET status=%d WHERE id=%d", value, id);
    return db_exec(db, sql);
}

int inventory_repo_find_all(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM inventorys WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

