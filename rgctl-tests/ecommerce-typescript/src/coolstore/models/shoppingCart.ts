import type { ShoppingCartItem } from './shoppingCartItem';

/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */
export interface ShoppingCart {
  cartId: string;
  cartItemTotal: number;
  cartItemPromoSavings: number;
  shippingTotal: number;
  shippingPromoSavings: number;
  cartTotal: number;
  shoppingCartItemList: ShoppingCartItem[];
}

export function createShoppingCart(cartId: string): ShoppingCart {
  return {
    cartId,
    cartItemTotal: 0,
    cartItemPromoSavings: 0,
    shippingTotal: 0,
    shippingPromoSavings: 0,
    cartTotal: 0,
    shoppingCartItemList: [],
  };
}

export function resetShoppingCartItemList(cart: ShoppingCart): void {
  cart.shoppingCartItemList = [];
}

export function addShoppingCartItem(cart: ShoppingCart, sci: ShoppingCartItem): void {
  cart.shoppingCartItemList.push(sci);
}

export function removeShoppingCartItem(cart: ShoppingCart, sci: ShoppingCartItem): boolean {
  const idx = cart.shoppingCartItemList.indexOf(sci);
  if (idx >= 0) {
    cart.shoppingCartItemList.splice(idx, 1);
    return true;
  }
  return false;
}
