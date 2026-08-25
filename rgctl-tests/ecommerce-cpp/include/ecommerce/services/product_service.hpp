#pragma once
#include <sqlite3.h>

namespace ecommerce::services {
int get_product(sqlite3* db, int id, void* out);
}  // namespace ecommerce::services
