import type { CoolstoreOrderItem } from './coolstoreOrderItem';

export interface CoolstoreOrder {
  orderId: number;
  cartId: string;
  cartTotal: number;
  items: CoolstoreOrderItem[];
}

export function createCoolstoreOrder(): CoolstoreOrder {
  return {
    orderId: 0,
    cartId: '',
    cartTotal: 0,
    items: [],
  };
}
