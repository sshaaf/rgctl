#include "ecommerce/services/inventory_service.hpp"
#include "ecommerce/repositories/inventory_repository.hpp"

namespace ecommerce::services {

int get_by_product(sqlite3* db, int product_id, void* out) {
    return repositories::inventory_find_by_id(db, product_id, out);
}

}  // namespace ecommerce::services
