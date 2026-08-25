#include "ecommerce/models/product.h"
#include <string.h>

void product_init(product_t *obj) { if (obj) { memset(obj, 0, sizeof(*obj)); } }

void product_set_price(product_t *p, double price) { if (p) p->price = price; }

int product_is_valid(const product_t *p) { return p && p->price > 0 && p->name[0]; }

