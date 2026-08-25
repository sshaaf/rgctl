#ifndef EC_COOLSTORE_PRODUCT_SERVICE_H
#define EC_COOLSTORE_PRODUCT_SERVICE_H

#include "ecommerce/coolstore/models/catalog_product.h"

#define COOLSTORE_MAX_PRODUCTS 16

void coolstore_product_service_init(void);
int coolstore_product_get_products(catalog_product_t *out, int cap, int *count);
const catalog_product_t *coolstore_product_get_by_item_id(const char *itemId);

#endif
