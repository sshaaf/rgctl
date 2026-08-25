#include "ecommerce/coolstore/services/coolstore_product_service.h"
#include <string.h>

static catalog_product_t g_catalog[COOLSTORE_MAX_PRODUCTS];
static int g_catalog_count;
static int g_product_inited;

static void seed(const char *id, const char *name, const char *desc, double price) {
    if (g_catalog_count >= COOLSTORE_MAX_PRODUCTS) return;
    catalog_product_set(&g_catalog[g_catalog_count++], id, name, desc, price);
}

void coolstore_product_service_init(void) {
    if (g_product_inited) return;
    g_catalog_count = 0;
    seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99);
    seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50);
    seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80);
    seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50);
    seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99);
    g_product_inited = 1;
}

int coolstore_product_get_products(catalog_product_t *out, int cap, int *count) {
    int i, n;
    coolstore_product_service_init();
    if (!count) return -1;
    n = g_catalog_count < cap ? g_catalog_count : cap;
    if (out) {
        for (i = 0; i < n; i++) out[i] = g_catalog[i];
    }
    *count = n;
    return 0;
}

const catalog_product_t *coolstore_product_get_by_item_id(const char *itemId) {
    int i;
    coolstore_product_service_init();
    if (!itemId) return NULL;
    for (i = 0; i < g_catalog_count; i++) {
        if (strcmp(g_catalog[i].itemId, itemId) == 0) return &g_catalog[i];
    }
    return NULL;
}
