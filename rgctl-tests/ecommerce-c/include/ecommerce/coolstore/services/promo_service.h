#ifndef EC_COOLSTORE_PROMO_SERVICE_H
#define EC_COOLSTORE_PROMO_SERVICE_H

#include "ecommerce/coolstore/models/shopping_cart.h"

void promo_apply_cart_item_promotions(shopping_cart_t *shoppingCart);
void promo_apply_shipping_promotions(shopping_cart_t *shoppingCart);

#endif
