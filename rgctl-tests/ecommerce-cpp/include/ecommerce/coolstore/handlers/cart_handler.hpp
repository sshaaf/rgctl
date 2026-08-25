#pragma once

namespace ecommerce::coolstore::handlers {

/**
 * Dispatch CoolStore cart routes:
 * GET /services/cart/{cartId}
 * POST /services/cart/{cartId}/{itemId}/{quantity}
 * DELETE /services/cart/{cartId}/{itemId}/{quantity}
 * POST /services/cart/checkout/{cartId}
 */
int handle_cart(const char* query);

}  // namespace ecommerce::coolstore::handlers
