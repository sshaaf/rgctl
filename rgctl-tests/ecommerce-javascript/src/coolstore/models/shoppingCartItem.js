function createShoppingCartItem() {
  return {
    price: 0,
    quantity: 0,
    promoSavings: 0,
    product: null,
  };
}

module.exports = { createShoppingCartItem };
