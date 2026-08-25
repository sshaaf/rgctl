#pragma once
#include <sqlite3.h>
namespace ecommerce::handlers {
int handle_cart(sqlite3* db, const char* query);
}  // namespace ecommerce::handlers
