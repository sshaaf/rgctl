#include "ecommerce/coolstore/models/shopping_cart_item.h"
#include <string.h>

void shopping_cart_item_init(shopping_cart_item_t *item) {
    if (item) {
        memset(item, 0, sizeof(*item));
    }
}
