#ifndef EC_COOLSTORE_ORDER_SERVICE_H
#define EC_COOLSTORE_ORDER_SERVICE_H

#include "ecommerce/coolstore/models/coolstore_order.h"
#include "ecommerce/coolstore/models/shopping_cart.h"

#define COOLSTORE_MAX_ORDERS 64

void coolstore_order_service_init(void);
coolstore_order_t *coolstore_order_process(shopping_cart_t *cart);
int coolstore_order_get_orders(coolstore_order_t *out, int cap, int *count);
const coolstore_order_t *coolstore_order_get_by_id(long orderId);

#endif
