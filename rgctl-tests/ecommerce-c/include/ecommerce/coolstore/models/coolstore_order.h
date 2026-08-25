#ifndef EC_COOLSTORE_ORDER_H
#define EC_COOLSTORE_ORDER_H

#include "ecommerce/coolstore/models/coolstore_order_item.h"

#define COOLSTORE_MAX_ORDER_ITEMS 32

typedef struct {
    long orderId;
    char cartId[64];
    double cartTotal;
    coolstore_order_item_t items[COOLSTORE_MAX_ORDER_ITEMS];
    int item_count;
} coolstore_order_t;

void coolstore_order_init(coolstore_order_t *order);

#endif
