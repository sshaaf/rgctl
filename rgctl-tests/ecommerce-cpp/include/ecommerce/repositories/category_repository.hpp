#pragma once
#include <sqlite3.h>
#include <cstddef>

namespace ecommerce::repositories {
int category_find_by_id(sqlite3* db, int id, void* out);
int category_create(sqlite3* db, const void* entity);
int category_find_all(sqlite3* db, void* out, std::size_t cap, int* count);
}  // namespace ecommerce::repositories
