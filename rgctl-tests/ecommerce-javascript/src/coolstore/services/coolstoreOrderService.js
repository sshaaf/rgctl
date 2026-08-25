const { createCoolstoreOrder } = require('../models/coolstoreOrder');
const { createCoolstoreOrderItem } = require('../models/coolstoreOrderItem');

class CoolstoreOrderService {
  constructor() {
    this.seq = 1;
    this.orders = new Map();
  }

  process(cart) {
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

  getOrders() {
    return Array.from(this.orders.values());
  }

  getOrderById(orderId) {
    return this.orders.get(orderId);
  }
}

module.exports = { CoolstoreOrderService };
