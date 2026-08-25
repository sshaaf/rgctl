#pragma once
#include <sqlite3.h>
#include <cstddef>

namespace ecommerce::services {
int get_user_cart(sqlite3* db, int user_id, void* out, std::size_t cap, int* count);
int add_item(sqlite3* db, int user_id, int product_id, int qty);
int clear_cart(sqlite3* db, int user_id);
}  // namespace ecommerce::services
