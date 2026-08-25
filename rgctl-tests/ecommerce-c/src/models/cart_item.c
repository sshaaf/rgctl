#include "ecommerce/models/cart_item.h"
#include <string.h>

void cart_item_init(cart_item_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

double cart_item_total_price(const cart_item_t *item, double unit_price) { return item ? item->quantity * unit_price : 0; }

