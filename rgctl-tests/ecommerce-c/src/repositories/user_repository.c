#include "ecommerce/repositories/user_repository.h"
#include "ecommerce/db/sqlite.h"
#include <stdio.h>
#include <string.h>

int user_repo_find_by_id(sqlite3 *db, int id, void *out) {
    if (!db || !out) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM users WHERE id=%d", id);
    return db_exec(db, sql);
}

int user_repo_find_by_email(sqlite3 *db, int id, void *out) {
    if (!db || !out) return -1;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT * FROM users WHERE id=%d", id);
    return db_exec(db, sql);
}

int user_repo_create(sqlite3 *db, const void *entity) {
    if (!db || !entity) return -1;
    return db_exec(db, "INSERT INTO users DEFAULT VALUES");
}

int user_repo_exists_by_email(sqlite3 *db, const char *email) {
    if (!db || !email) return 0;
    char sql[256];
    snprintf(sql, sizeof(sql), "SELECT 1 FROM users WHERE email='%s'", email);
    db_exec(db, sql);
    return 0;
}

