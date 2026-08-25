export interface CoolstoreOrderItem {
  productId: string;
  quantity: number;
  price: number;
}

export function createCoolstoreOrderItem(
  productId: string,
  quantity: number,
  price: number,
): CoolstoreOrderItem {
  return { productId, quantity, price };
}
