class PromoService {
  constructor() {
    this.percentOffByItem = new Map([['329299', 0.25]]);
  }

  applyCartItemPromotions(shoppingCart) {
    if (!shoppingCart.shoppingCartItemList.length) {
      return;
    }
    for (const sci of shoppingCart.shoppingCartItemList) {
      if (!sci.product) {
        continue;
      }
      const pct = this.percentOffByItem.get(sci.product.itemId);
      if (pct != null) {
        sci.promoSavings = sci.product.price * pct * -1;
        sci.price = sci.product.price * (1 - pct);
      }
    }
  }

  applyShippingPromotions(shoppingCart) {
    if (shoppingCart.cartItemTotal >= 75) {
      shoppingCart.shippingPromoSavings = shoppingCart.shippingTotal * -1;
      shoppingCart.shippingTotal = 0;
    }
  }
}

module.exports = { PromoService };
