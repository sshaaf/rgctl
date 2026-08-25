#pragma once
#include <sqlite3.h>

namespace ecommerce::services {
int get_by_product(sqlite3* db, int product_id, void* out);
}  // namespace ecommerce::services
