#include "ecommerce/coolstore/services/shopping_cart_service.h"
#include "ecommerce/coolstore/services/coolstore_order_service.h"
#include "ecommerce/coolstore/services/coolstore_product_service.h"
#include "ecommerce/coolstore/services/promo_service.h"
#include "ecommerce/coolstore/services/shipping_service.h"
#include <string.h>

#define COOLSTORE_MAX_CARTS 32

static shopping_cart_t g_carts[COOLSTORE_MAX_CARTS];
static int g_cart_count;
static int g_cart_svc_inited;

void shopping_cart_service_init(void) {
    if (g_cart_svc_inited) return;
    coolstore_product_service_init();
    coolstore_order_service_init();
    memset(g_carts, 0, sizeof(g_carts));
    g_cart_count = 0;
    g_cart_svc_inited = 1;
}

shopping_cart_t *shopping_cart_service_get(const char *cartId) {
    int i;
    shopping_cart_service_init();
    if (!cartId) return NULL;
    for (i = 0; i < g_cart_count; i++) {
        if (strcmp(g_carts[i].cartId, cartId) == 0) return &g_carts[i];
    }
    if (g_cart_count >= COOLSTORE_MAX_CARTS) return NULL;
    shopping_cart_init(&g_carts[g_cart_count], cartId);
    return &g_carts[g_cart_count++];
}

const catalog_product_t *shopping_cart_service_get_product(const char *itemId) {
    return coolstore_product_get_by_item_id(itemId);
}

static void init_shopping_cart_for_pricing(shopping_cart_t *sc) {
    int i;
    sc->cartItemTotal = 0;
    sc->cartItemPromoSavings = 0;
    sc->shippingTotal = 0;
    sc->shippingPromoSavings = 0;
    sc->cartTotal = 0;

    for (i = 0; i < sc->item_count; i++) {
        shopping_cart_item_t *sci = &sc->shoppingCartItemList[i];
        if (sci->has_product) {
            const catalog_product_t *p = shopping_cart_service_get_product(sci->product.itemId);
            if (p) {
                sci->product = *p;
                sci->price = p->price;
            }
        }
        sci->promoSavings = 0;
    }
}

/** Mutates ShoppingCart totals — primary CPG field-write site. */
void price_shopping_cart(shopping_cart_t *sc) {
    int i;
    if (!sc) return;
    init_shopping_cart_for_pricing(sc);

    if (sc->item_count > 0) {
        promo_apply_cart_item_promotions(sc);

        for (i = 0; i < sc->item_count; i++) {
            shopping_cart_item_t *sci = &sc->shoppingCartItemList[i];
            sc->cartItemPromoSavings =
                sc->cartItemPromoSavings + sci->promoSavings * sci->quantity;
            sc->cartItemTotal = sc->cartItemTotal + sci->price * sci->quantity;
        }

        sc->shippingTotal = shipping_calculate(sc);
        if (sc->cartItemTotal >= 25) {
            sc->shippingTotal = sc->shippingTotal + shipping_calculate_insurance(sc);
        }
    }

    promo_apply_shipping_promotions(sc);
    sc->cartTotal = sc->cartItemTotal + sc->shippingTotal;
}

shopping_cart_t *shopping_cart_service_checkout(const char *cartId) {
    shopping_cart_t *cart = shopping_cart_service_get(cartId);
    if (!cart) return NULL;
    price_shopping_cart(cart);
    coolstore_order_process(cart);
    shopping_cart_reset_items(cart);
    price_shopping_cart(cart);
    return cart;
}

int shopping_cart_service_dedupe_items(shopping_cart_t *cart) {
    char ids[COOLSTORE_MAX_CART_ITEMS][32];
    int qtys[COOLSTORE_MAX_CART_ITEMS];
    int n = 0;
    int i, j;
    shopping_cart_item_t rebuilt[COOLSTORE_MAX_CART_ITEMS];
    int rebuilt_count = 0;

    if (!cart) return -1;

    for (i = 0; i < cart->item_count; i++) {
        shopping_cart_item_t *sci = &cart->shoppingCartItemList[i];
        int found = 0;
        if (!sci->has_product) continue;
        for (j = 0; j < n; j++) {
            if (strcmp(ids[j], sci->product.itemId) == 0) {
                qtys[j] += sci->quantity;
                found = 1;
                break;
            }
        }
        if (!found && n < COOLSTORE_MAX_CART_ITEMS) {
            strncpy(ids[n], sci->product.itemId, sizeof(ids[n]) - 1);
            qtys[n] = sci->quantity;
            n++;
        }
    }

    for (i = 0; i < n; i++) {
        const catalog_product_t *p = shopping_cart_service_get_product(ids[i]);
        shopping_cart_item_t item;
        if (!p) continue;
        shopping_cart_item_init(&item);
        item.quantity = qtys[i];
        item.price = p->price;
        item.product = *p;
        item.has_product = 1;
        rebuilt[rebuilt_count++] = item;
    }

    shopping_cart_reset_items(cart);
    for (i = 0; i < rebuilt_count; i++) {
        shopping_cart_add_item(cart, &rebuilt[i]);
    }
    return 0;
}
