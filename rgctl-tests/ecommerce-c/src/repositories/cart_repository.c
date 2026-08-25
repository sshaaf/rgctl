#include "ecommerce/repositories/cart_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int cart_repo_find_by_user(sqlite3 *db, int key, void *out, size_t out_cap, int *count) {
    if (!db || !count) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM carts WHERE id = %d", key);
    db_exec(db, sql);
    *count = 0;
    return 0;
}

int cart_repo_add_item(sqlite3 *db, int user_id, int product_id, int qty) {
    if (!db) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "INSERT INTO cart_items VALUES(%d,%d,%d)", user_id, product_id, qty);
    return db_exec(db, sql);
}

int cart_repo_clear(sqlite3 *db, int user_id) {
    if (!db) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "DELETE FROM cart_items WHERE user_id=%d", user_id);
    return db_exec(db, sql);
}

int cart_repo_count(sqlite3 *db, int *count) {
    if (!db || !count) return -1;
    *count = 0;
    db_exec(db, "SELECT COUNT(*) FROM carts");
    return 0;
}

