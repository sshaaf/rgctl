#ifndef EC_ORDER_H
#define EC_ORDER_H
#include "ecommerce/types.h"
void order_init(order_t *obj);
void order_add_total(order_t *order, double amount);
int order_is_paid(const order_t *obj);
#endif
