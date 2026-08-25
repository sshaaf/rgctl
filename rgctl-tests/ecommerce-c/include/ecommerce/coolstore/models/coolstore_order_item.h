#ifndef EC_COOLSTORE_ORDER_ITEM_H
#define EC_COOLSTORE_ORDER_ITEM_H

typedef struct {
    char productId[32];
    int quantity;
    double price;
} coolstore_order_item_t;

void coolstore_order_item_init(coolstore_order_item_t *item, const char *productId,
                               int quantity, double price);

#endif
