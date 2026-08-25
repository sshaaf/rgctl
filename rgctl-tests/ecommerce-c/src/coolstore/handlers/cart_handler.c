#include "ecommerce/coolstore/handlers/cart_handler.h"
#include "ecommerce/coolstore/services/shopping_cart_service.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *skip_method(const char *query) {
    const char *p = query;
    if (!p) return NULL;
    if (strncmp(p, "GET ", 4) == 0) return p + 4;
    if (strncmp(p, "POST ", 5) == 0) return p + 5;
    if (strncmp(p, "DELETE ", 7) == 0) return p + 7;
    return p;
}

static int is_delete(const char *query) {
    return query && (strncmp(query, "DELETE ", 7) == 0 || strstr(query, "DELETE") == query);
}

static int is_post(const char *query) {
    return query && (strncmp(query, "POST ", 5) == 0 || strstr(query, "POST") == query);
}

static shopping_cart_t *coolstore_cart_add(const char *cartId, const char *itemId, int quantity) {
    shopping_cart_t *cart;
    const catalog_product_t *product;
    shopping_cart_item_t sci;

    cart = shopping_cart_service_get(cartId);
    if (!cart) return NULL;
    product = shopping_cart_service_get_product(itemId);
    if (!product) return cart;

    shopping_cart_item_init(&sci);
    sci.product = *product;
    sci.has_product = 1;
    sci.quantity = quantity;
    sci.price = product->price;
    shopping_cart_add_item(cart, &sci);
    price_shopping_cart(cart);
    shopping_cart_service_dedupe_items(cart);
    price_shopping_cart(cart);
    return cart;
}

static shopping_cart_t *coolstore_cart_delete(const char *cartId, const char *itemId, int quantity) {
    shopping_cart_t *cart;
    int i;

    cart = shopping_cart_service_get(cartId);
    if (!cart) return NULL;

    for (i = cart->item_count - 1; i >= 0; i--) {
        shopping_cart_item_t *sci = &cart->shoppingCartItemList[i];
        if (!sci->has_product || strcmp(sci->product.itemId, itemId) != 0) continue;
        if (quantity >= sci->quantity) {
            shopping_cart_remove_item_at(cart, i);
        } else {
            sci->quantity -= quantity;
        }
    }
    price_shopping_cart(cart);
    return cart;
}

int handle_coolstore_cart(const char *query) {
    const char *path;
    char cartId[64];
    char itemId[32];
    int quantity = 0;

    if (!query) return -1;
    path = skip_method(query);
    if (!path || !strstr(path, "/services/cart")) return 0;

    /* POST /services/cart/checkout/{cartId} */
    if (strstr(path, "/services/cart/checkout/") != NULL) {
        const char *id = strstr(path, "/services/cart/checkout/") + strlen("/services/cart/checkout/");
        if (!shopping_cart_service_checkout(id)) return -1;
        return 0;
    }

    /* POST|DELETE /services/cart/{cartId}/{itemId}/{quantity} */
    if (sscanf(path, "/services/cart/%63[^/]/%31[^/]/%d", cartId, itemId, &quantity) == 3) {
        if (is_delete(query)) {
            return coolstore_cart_delete(cartId, itemId, quantity) ? 0 : -1;
        }
        if (is_post(query) || strstr(query, "add") != NULL) {
            return coolstore_cart_add(cartId, itemId, quantity) ? 0 : -1;
        }
        /* default treat as add when method omitted */
        return coolstore_cart_add(cartId, itemId, quantity) ? 0 : -1;
    }

    /* GET /services/cart/{cartId} */
    if (sscanf(path, "/services/cart/%63s", cartId) == 1) {
        return shopping_cart_service_get(cartId) ? 0 : -1;
    }
    return 0;
}
