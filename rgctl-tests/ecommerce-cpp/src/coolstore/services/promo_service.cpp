#include "ecommerce/coolstore/services/promo_service.hpp"

namespace ecommerce::coolstore {

PromoService::PromoService() { percentOffByItem_["329299"] = 0.25; }

void PromoService::applyCartItemPromotions(ShoppingCart& shoppingCart) {
    if (shoppingCart.shoppingCartItemList.empty()) return;
    for (auto& sci : shoppingCart.shoppingCartItemList) {
        if (!sci.hasProduct) continue;
        auto it = percentOffByItem_.find(sci.product.itemId);
        if (it != percentOffByItem_.end()) {
            double pct = it->second;
            sci.promoSavings = sci.product.price * pct * -1;
            sci.price = sci.product.price * (1 - pct);
        }
    }
}

void PromoService::applyShippingPromotions(ShoppingCart& shoppingCart) {
    if (shoppingCart.cartItemTotal >= 75) {
        shoppingCart.shippingPromoSavings = shoppingCart.shippingTotal * -1;
        shoppingCart.shippingTotal = 0;
    }
}

}  // namespace ecommerce::coolstore
