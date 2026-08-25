#ifndef EC_COOLSTORE_SHOPPING_CART_ITEM_H
#define EC_COOLSTORE_SHOPPING_CART_ITEM_H

#include "ecommerce/coolstore/models/catalog_product.h"

typedef struct {
    double price;
    int quantity;
    double promoSavings;
    catalog_product_t product;
    int has_product;
} shopping_cart_item_t;

void shopping_cart_item_init(shopping_cart_item_t *item);

#endif
