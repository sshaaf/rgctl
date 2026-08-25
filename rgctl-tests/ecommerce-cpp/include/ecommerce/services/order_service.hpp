#pragma once
#include <sqlite3.h>

namespace ecommerce::services {
int checkout(sqlite3* db, int user_id);
int get_order(sqlite3* db, int id, void* out);
}  // namespace ecommerce::services
