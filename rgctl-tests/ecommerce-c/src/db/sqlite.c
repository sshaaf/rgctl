#include "ecommerce/db/sqlite.h"
#include <stdio.h>

int db_open(const char *path, sqlite3 **db) {
    if (!path || !db) return -1;
    return sqlite3_open(path, db);
}

int db_close(sqlite3 *db) {
    if (!db) return -1;
    return sqlite3_close(db);
}

int db_exec(sqlite3 *db, const char *sql) {
    if (!db || !sql) return -1;
    char *err = NULL;
    int rc = sqlite3_exec(db, sql, NULL, NULL, &err);
    if (err) { fprintf(stderr, "sql error: %s\n", err); sqlite3_free(err); }
    return rc;
}
