#ifndef EC_COOLSTORE_CATALOG_PRODUCT_H
#define EC_COOLSTORE_CATALOG_PRODUCT_H

typedef struct {
    char itemId[32];
    char name[128];
    char desc[256];
    double price;
} catalog_product_t;

void catalog_product_init(catalog_product_t *p);
void catalog_product_set(catalog_product_t *p, const char *itemId, const char *name,
                         const char *desc, double price);

#endif
