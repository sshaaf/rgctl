#pragma once
#include <sqlite3.h>
namespace ecommerce::handlers {
int handle_auth(sqlite3* db, const char* query);
}  // namespace ecommerce::handlers
