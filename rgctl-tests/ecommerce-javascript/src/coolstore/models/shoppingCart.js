/** CoolStore-shaped cart with mutable pricing totals (CPG field-write target). */

function createShoppingCart(cartId) {
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

function resetShoppingCartItemList(cart) {
  cart.shoppingCartItemList = [];
}

function addShoppingCartItem(cart, sci) {
  cart.shoppingCartItemList.push(sci);
}

function removeShoppingCartItem(cart, sci) {
  const idx = cart.shoppingCartItemList.indexOf(sci);
  if (idx >= 0) {
    cart.shoppingCartItemList.splice(idx, 1);
    return true;
  }
  return false;
}

module.exports = {
  createShoppingCart,
  resetShoppingCartItemList,
  addShoppingCartItem,
  removeShoppingCartItem,
};
