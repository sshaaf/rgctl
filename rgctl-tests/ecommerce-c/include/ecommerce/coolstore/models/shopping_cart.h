#ifndef EC_COOLSTORE_SHOPPING_CART_H
#define EC_COOLSTORE_SHOPPING_CART_H

#include "ecommerce/coolstore/models/shopping_cart_item.h"

#define COOLSTORE_MAX_CART_ITEMS 32

/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */
typedef struct {
    char cartId[64];
    double cartItemTotal;
    double cartItemPromoSavings;
    double shippingTotal;
    double shippingPromoSavings;
    double cartTotal;
    shopping_cart_item_t shoppingCartItemList[COOLSTORE_MAX_CART_ITEMS];
    int item_count;
} shopping_cart_t;

void shopping_cart_init(shopping_cart_t *cart, const char *cartId);
void shopping_cart_reset_items(shopping_cart_t *cart);
int shopping_cart_add_item(shopping_cart_t *cart, const shopping_cart_item_t *item);
int shopping_cart_remove_item_at(shopping_cart_t *cart, int index);

#endif
