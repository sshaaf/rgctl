#pragma once

namespace ecommerce::coolstore::handlers {

/** Dispatch GET /services/products and GET /services/products/{itemId}. */
int handle_products(const char* query);

}  // namespace ecommerce::coolstore::handlers
