#ifndef EC_COOLSTORE_SHOPPING_CART_SERVICE_H
#define EC_COOLSTORE_SHOPPING_CART_SERVICE_H

#include "ecommerce/coolstore/models/catalog_product.h"
#include "ecommerce/coolstore/models/shopping_cart.h"

void shopping_cart_service_init(void);
shopping_cart_t *shopping_cart_service_get(const char *cartId);
const catalog_product_t *shopping_cart_service_get_product(const char *itemId);
shopping_cart_t *shopping_cart_service_checkout(const char *cartId);
/** Mutates ShoppingCart totals — primary CPG field-write site. */
void price_shopping_cart(shopping_cart_t *sc);
int shopping_cart_service_dedupe_items(shopping_cart_t *cart);

#endif
