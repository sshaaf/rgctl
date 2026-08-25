#include "ecommerce/coolstore/services/coolstore_order_service.h"
#include <string.h>

static coolstore_order_t g_orders[COOLSTORE_MAX_ORDERS];
static int g_order_count;
static long g_order_seq = 1;
static int g_order_inited;

void coolstore_order_service_init(void) {
    if (g_order_inited) return;
    memset(g_orders, 0, sizeof(g_orders));
    g_order_count = 0;
    g_order_seq = 1;
    g_order_inited = 1;
}

coolstore_order_t *coolstore_order_process(shopping_cart_t *cart) {
    coolstore_order_t *order;
    int i;
    coolstore_order_service_init();
    if (!cart || g_order_count >= COOLSTORE_MAX_ORDERS) return NULL;
    order = &g_orders[g_order_count++];
    coolstore_order_init(order);
    order->orderId = g_order_seq++;
    strncpy(order->cartId, cart->cartId, sizeof(order->cartId) - 1);
    order->cartTotal = cart->cartTotal;
    for (i = 0; i < cart->item_count && order->item_count < COOLSTORE_MAX_ORDER_ITEMS; i++) {
        shopping_cart_item_t *sci = &cart->shoppingCartItemList[i];
        if (!sci->has_product) continue;
        coolstore_order_item_init(&order->items[order->item_count], sci->product.itemId,
                                  sci->quantity, sci->price);
        order->item_count++;
    }
    return order;
}

int coolstore_order_get_orders(coolstore_order_t *out, int cap, int *count) {
    int i, n;
    coolstore_order_service_init();
    if (!count) return -1;
    n = g_order_count < cap ? g_order_count : cap;
    if (out) {
        for (i = 0; i < n; i++) out[i] = g_orders[i];
    }
    *count = n;
    return 0;
}

const coolstore_order_t *coolstore_order_get_by_id(long orderId) {
    int i;
    coolstore_order_service_init();
    for (i = 0; i < g_order_count; i++) {
        if (g_orders[i].orderId == orderId) return &g_orders[i];
    }
    return NULL;
}
