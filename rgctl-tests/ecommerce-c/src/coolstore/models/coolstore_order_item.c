#include "ecommerce/coolstore/models/coolstore_order_item.h"
#include <string.h>

void coolstore_order_item_init(coolstore_order_item_t *item, const char *productId,
                               int quantity, double price) {
    if (!item) return;
    memset(item, 0, sizeof(*item));
    if (productId) strncpy(item->productId, productId, sizeof(item->productId) - 1);
    item->quantity = quantity;
    item->price = price;
}
