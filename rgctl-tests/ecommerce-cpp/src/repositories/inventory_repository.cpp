#include "ecommerce/repositories/inventory_repository.hpp"
#include "ecommerce/db/sqlite.hpp"
#include <cstdio>

namespace ecommerce::repositories {

int inventory_find_by_id(sqlite3* db, int id, void* out) {
    if (!db || !out) return -1;
    char sql[256];
    std::snprintf(sql, sizeof(sql), "SELECT * FROM inventorys WHERE id=%d", id);
    return db::exec(db, sql);
}

int inventory_create(sqlite3* db, const void* entity) {
    if (!db || !entity) return -1;
    return db::exec(db, "INSERT INTO inventorys DEFAULT VALUES");
}

int inventory_find_all(sqlite3* db, void* out, std::size_t cap, int* count) {
    (void)out; (void)cap;
    if (!db || !count) return -1;
    *count = 0;
    return db::exec(db, "SELECT * FROM inventorys");
}

}  // namespace ecommerce::repositories
