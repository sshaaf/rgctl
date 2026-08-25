#pragma once
#include <sqlite3.h>
namespace ecommerce::handlers {
int handle_health(sqlite3* db, const char* query);
}  // namespace ecommerce::handlers
