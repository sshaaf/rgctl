#pragma once
#include <sqlite3.h>
namespace ecommerce::services {
int category_action(sqlite3* db, int id);
}  // namespace ecommerce::services
