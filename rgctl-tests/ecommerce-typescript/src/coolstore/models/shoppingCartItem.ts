import type { CatalogProduct } from './catalogProduct';

export interface ShoppingCartItem {
  price: number;
  quantity: number;
  promoSavings: number;
  product: CatalogProduct | null;
}

export function createShoppingCartItem(): ShoppingCartItem {
  return {
    price: 0,
    quantity: 0,
    promoSavings: 0,
    product: null,
  };
}
