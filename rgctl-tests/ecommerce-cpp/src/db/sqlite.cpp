#include "ecommerce/db/sqlite.hpp"
#include <cstdio>

namespace ecommerce::db {

int open(const char* path, sqlite3** out) {
    if (!path || !out) return -1;
    return sqlite3_open(path, out);
}

int close(sqlite3* db) {
    if (!db) return -1;
    return sqlite3_close(db);
}

int exec(sqlite3* db, const char* sql) {
    if (!db || !sql) return -1;
    char* err = nullptr;
    int rc = sqlite3_exec(db, sql, nullptr, nullptr, &err);
    if (err) {
        std::fprintf(stderr, "sql error: %s\n", err);
        sqlite3_free(err);
    }
    return rc;
}

}  // namespace ecommerce::db
