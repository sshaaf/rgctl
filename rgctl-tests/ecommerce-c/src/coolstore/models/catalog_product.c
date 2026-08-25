#include "ecommerce/coolstore/models/catalog_product.h"
#include <string.h>

void catalog_product_init(catalog_product_t *p) {
    if (p) {
        memset(p, 0, sizeof(*p));
    }
}

void catalog_product_set(catalog_product_t *p, const char *itemId, const char *name,
                         const char *desc, double price) {
    if (!p) return;
    catalog_product_init(p);
    if (itemId) strncpy(p->itemId, itemId, sizeof(p->itemId) - 1);
    if (name) strncpy(p->name, name, sizeof(p->name) - 1);
    if (desc) strncpy(p->desc, desc, sizeof(p->desc) - 1);
    p->price = price;
}
