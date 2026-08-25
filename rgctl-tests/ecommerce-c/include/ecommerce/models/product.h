#ifndef EC_PRODUCT_H
#define EC_PRODUCT_H
#include "ecommerce/types.h"
void product_init(product_t *obj);
void product_set_price(product_t *p, double price);
int product_is_valid(const product_t *obj);
#endif
