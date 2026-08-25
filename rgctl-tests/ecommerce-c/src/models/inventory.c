#include "ecommerce/models/inventory.h"
#include <string.h>

void inventory_init(inventory_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

int inventory_reserve(inventory_t *inv, int qty) { if (!inv || qty > inv->quantity) return 0; inv->quantity -= qty; return 1; }

int inventory_release(inventory_t *inv, int qty) { if (!inv) return 0; inv->quantity += qty; return 1; }

