#ifndef EC_COOLSTORE_CART_HANDLER_H
#define EC_COOLSTORE_CART_HANDLER_H

/**
 * Dispatch CoolStore cart routes:
 * GET /services/cart/{cartId}
 * POST /services/cart/{cartId}/{itemId}/{quantity}
 * DELETE /services/cart/{cartId}/{itemId}/{quantity}
 * POST /services/cart/checkout/{cartId}
 */
int handle_coolstore_cart(const char *query);

#endif
