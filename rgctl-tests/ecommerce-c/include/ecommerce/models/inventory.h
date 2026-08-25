#ifndef EC_INVENTORY_H
#define EC_INVENTORY_H
#include "ecommerce/types.h"
void inventory_init(inventory_t *obj);
int inventory_reserve(inventory_t *inv, int qty);
int inventory_release(inventory_t *inv, int qty);
#endif
