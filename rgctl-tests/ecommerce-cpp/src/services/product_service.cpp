#include "ecommerce/services/product_service.hpp"
#include "ecommerce/repositories/product_repository.hpp"

namespace ecommerce::services {

int get_product(sqlite3* db, int id, void* out) {
    return repositories::product_find_by_id(db, id, out);
}

}  // namespace ecommerce::services
