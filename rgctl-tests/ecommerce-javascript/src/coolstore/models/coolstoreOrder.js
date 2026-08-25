function createCoolstoreOrder() {
  return {
    orderId: 0,
    cartId: '',
    cartTotal: 0,
    items: [],
  };
}

module.exports = { createCoolstoreOrder };
