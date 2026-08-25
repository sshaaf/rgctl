/** Lightweight CoolStore catalog product (itemId keyed). */
export interface CatalogProduct {
  itemId: string;
  name: string;
  desc: string;
  price: number;
}

export function createCatalogProduct(
  itemId: string,
  name: string,
  desc: string,
  price: number,
): CatalogProduct {
  return { itemId, name, desc, price };
}
