#include "ecommerce/services/cart_service.hpp"
#include "ecommerce/repositories/cart_repository.hpp"
#include "ecommerce/services/product_service.hpp"
#include "ecommerce/types.hpp"

namespace ecommerce::services {

int get_user_cart(sqlite3* db, int user_id, void* out, std::size_t cap, int* count) {
    return repositories::cart_find_all(db, out, cap, count);
}

int add_item(sqlite3* db, int user_id, int product_id, int qty) {
    Product p{};
    if (get_product(db, product_id, &p) != 0) return -2;
    return repositories::cart_create(db, nullptr);
}

int clear_cart(sqlite3* db, int user_id) {
    (void)user_id;
    return repositories::cart_create(db, nullptr);
}

}  // namespace ecommerce::services
