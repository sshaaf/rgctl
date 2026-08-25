#include "ecommerce/coolstore/services/promo_service.h"
#include <string.h>

static double percent_off_for_item(const char *itemId) {
    if (itemId && strcmp(itemId, "329299") == 0) return 0.25;
    return -1.0;
}

void promo_apply_cart_item_promotions(shopping_cart_t *shoppingCart) {
    int i;
    if (!shoppingCart || shoppingCart->item_count == 0) return;
    for (i = 0; i < shoppingCart->item_count; i++) {
        shopping_cart_item_t *sci = &shoppingCart->shoppingCartItemList[i];
        double pct;
        if (!sci->has_product) continue;
        pct = percent_off_for_item(sci->product.itemId);
        if (pct >= 0) {
            sci->promoSavings = sci->product.price * pct * -1;
            sci->price = sci->product.price * (1 - pct);
        }
    }
}

void promo_apply_shipping_promotions(shopping_cart_t *shoppingCart) {
    if (!shoppingCart) return;
    if (shoppingCart->cartItemTotal >= 75) {
        shoppingCart->shippingPromoSavings = shoppingCart->shippingTotal * -1;
        shoppingCart->shippingTotal = 0;
    }
}
