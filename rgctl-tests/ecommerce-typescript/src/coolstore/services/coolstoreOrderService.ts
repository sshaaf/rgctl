import { createCoolstoreOrder, type CoolstoreOrder } from '../models/coolstoreOrder';
import { createCoolstoreOrderItem } from '../models/coolstoreOrderItem';
import type { ShoppingCart } from '../models/shoppingCart';

export class CoolstoreOrderService {
  private seq = 1;
  private readonly orders = new Map<number, CoolstoreOrder>();

  process(cart: ShoppingCart): CoolstoreOrder {
    const order = createCoolstoreOrder();
    order.orderId = this.seq++;
    order.cartId = cart.cartId;
    order.cartTotal = cart.cartTotal;
    order.items = [];
    for (const sci of cart.shoppingCartItemList) {
      if (sci.product) {
        order.items.push(
          createCoolstoreOrderItem(sci.product.itemId, sci.quantity, sci.price),
        );
      }
    }
    this.orders.set(order.orderId, order);
    return order;
  }

  getOrders(): CoolstoreOrder[] {
    return Array.from(this.orders.values());
  }

  getOrderById(orderId: number): CoolstoreOrder | undefined {
    return this.orders.get(orderId);
  }
}
