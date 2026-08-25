#include "ecommerce/coolstore/handlers/product_handler.h"
#include "ecommerce/coolstore/services/coolstore_product_service.h"
#include <stdio.h>
#include <string.h>

static const char *skip_method(const char *query) {
    const char *p = query;
    if (!p) return NULL;
    if (strncmp(p, "GET ", 4) == 0) return p + 4;
    if (strncmp(p, "POST ", 5) == 0) return p + 5;
    if (strncmp(p, "DELETE ", 7) == 0) return p + 7;
    return p;
}

int handle_coolstore_products(const char *query) {
    const char *path;
    catalog_product_t products[COOLSTORE_MAX_PRODUCTS];
    int count = 0;
    const catalog_product_t *one;

    if (!query) return -1;
    path = skip_method(query);
    if (!path || strstr(path, "/services/products") != path) {
        if (!path || !strstr(path, "/services/products")) return 0;
    }

    /* GET /services/products/{itemId} */
    if (strncmp(path, "/services/products/", 19) == 0) {
        const char *itemId = path + 19;
        one = coolstore_product_get_by_item_id(itemId);
        return one ? 0 : -1;
    }

    /* GET /services/products */
    if (strcmp(path, "/services/products") == 0 || strstr(query, "/services/products") != NULL) {
        return coolstore_product_get_products(products, COOLSTORE_MAX_PRODUCTS, &count);
    }
    return 0;
}
