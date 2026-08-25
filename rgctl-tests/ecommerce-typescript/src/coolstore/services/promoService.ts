import type { ShoppingCart } from '../models/shoppingCart';

export class PromoService {
  private readonly percentOffByItem = new Map<string, number>([['329299', 0.25]]);

  applyCartItemPromotions(shoppingCart: ShoppingCart): void {
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

  applyShippingPromotions(shoppingCart: ShoppingCart): void {
    if (shoppingCart.cartItemTotal >= 75) {
      shoppingCart.shippingPromoSavings = shoppingCart.shippingTotal * -1;
      shoppingCart.shippingTotal = 0;
    }
  }
}
