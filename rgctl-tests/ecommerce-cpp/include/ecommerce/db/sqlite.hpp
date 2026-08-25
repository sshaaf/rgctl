#pragma once
#include <sqlite3.h>

namespace ecommerce::db {
int open(const char* path, sqlite3** out);
int close(sqlite3* db);
int exec(sqlite3* db, const char* sql);
}  // namespace ecommerce::db
