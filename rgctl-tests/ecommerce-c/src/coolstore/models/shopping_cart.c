#include "ecommerce/coolstore/models/shopping_cart.h"
#include <string.h>

void shopping_cart_init(shopping_cart_t *cart, const char *cartId) {
    if (!cart) return;
    memset(cart, 0, sizeof(*cart));
    if (cartId) {
        strncpy(cart->cartId, cartId, sizeof(cart->cartId) - 1);
    }
}

void shopping_cart_reset_items(shopping_cart_t *cart) {
    if (!cart) return;
    memset(cart->shoppingCartItemList, 0, sizeof(cart->shoppingCartItemList));
    cart->item_count = 0;
}

int shopping_cart_add_item(shopping_cart_t *cart, const shopping_cart_item_t *item) {
    if (!cart || !item || cart->item_count >= COOLSTORE_MAX_CART_ITEMS) return -1;
    cart->shoppingCartItemList[cart->item_count++] = *item;
    return 0;
}

int shopping_cart_remove_item_at(shopping_cart_t *cart, int index) {
    int i;
    if (!cart || index < 0 || index >= cart->item_count) return -1;
    for (i = index; i < cart->item_count - 1; i++) {
        cart->shoppingCartItemList[i] = cart->shoppingCartItemList[i + 1];
    }
    cart->item_count--;
    memset(&cart->shoppingCartItemList[cart->item_count], 0, sizeof(shopping_cart_item_t));
    return 0;
}
