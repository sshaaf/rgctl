#ifndef EC_CART_ITEM_H
#define EC_CART_ITEM_H
#include "ecommerce/types.h"
void cart_item_init(cart_item_t *obj);
double cart_item_total_price(const cart_item_t *item, double unit_price);
#endif
