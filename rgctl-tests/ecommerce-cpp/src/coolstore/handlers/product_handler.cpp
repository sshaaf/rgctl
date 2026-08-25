#include "ecommerce/coolstore/handlers/product_handler.hpp"
#include "ecommerce/coolstore/runtime.hpp"
#include <cstring>
#include <string>

namespace ecommerce::coolstore::handlers {
namespace {

const char* skip_method(const char* query) {
    if (!query) return nullptr;
    if (std::strncmp(query, "GET ", 4) == 0) return query + 4;
    if (std::strncmp(query, "POST ", 5) == 0) return query + 5;
    if (std::strncmp(query, "DELETE ", 7) == 0) return query + 7;
    return query;
}

}  // namespace

int handle_products(const char* query) {
    if (!query) return -1;
    const char* path = skip_method(query);
    if (!path || std::strstr(path, "/services/products") == nullptr) return 0;

    auto& products = runtime().productService;
    if (std::strncmp(path, "/services/products/", 19) == 0) {
        std::string itemId = path + 19;
        return products.getProductByItemId(itemId) ? 0 : -1;
    }
    (void)products.getProducts();
    return 0;
}

}  // namespace ecommerce::coolstore::handlers
