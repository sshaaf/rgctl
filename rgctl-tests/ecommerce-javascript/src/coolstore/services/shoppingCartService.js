const {
  addShoppingCartItem,
  createShoppingCart,
  removeShoppingCartItem,
  resetShoppingCartItemList,
} = require('../models/shoppingCart');
const { createShoppingCartItem } = require('../models/shoppingCartItem');

class ShoppingCartService {
  constructor(productService, promoService, shippingService, orderService) {
    this.productService = productService;
    this.promoService = promoService;
    this.shippingService = shippingService;
    this.orderService = orderService;
    this.carts = new Map();
  }

  getShoppingCart(cartId) {
    let cart = this.carts.get(cartId);
    if (!cart) {
      cart = createShoppingCart(cartId);
      this.carts.set(cartId, cart);
    }
    return cart;
  }

  getProduct(itemId) {
    return this.productService.getProductByItemId(itemId);
  }

  checkOutShoppingCart(cartId) {
    const cart = this.getShoppingCart(cartId);
    this.priceShoppingCart(cart);
    this.orderService.process(cart);
    resetShoppingCartItemList(cart);
    this.priceShoppingCart(cart);
    return cart;
  }

  /** Mutates ShoppingCart totals — primary CPG field-write site. */
  priceShoppingCart(sc) {
    if (!sc) {
      return;
    }
    this.initShoppingCartForPricing(sc);

    if (sc.shoppingCartItemList && sc.shoppingCartItemList.length > 0) {
      this.promoService.applyCartItemPromotions(sc);

      for (const sci of sc.shoppingCartItemList) {
        sc.cartItemPromoSavings += sci.promoSavings * sci.quantity;
        sc.cartItemTotal += sci.price * sci.quantity;
      }

      sc.shippingTotal = this.shippingService.calculateShipping(sc);
      if (sc.cartItemTotal >= 25) {
        sc.shippingTotal += this.shippingService.calculateShippingInsurance(sc);
      }
    }

    this.promoService.applyShippingPromotions(sc);
    sc.cartTotal = sc.cartItemTotal + sc.shippingTotal;
  }

  initShoppingCartForPricing(sc) {
    sc.cartItemTotal = 0;
    sc.cartItemPromoSavings = 0;
    sc.shippingTotal = 0;
    sc.shippingPromoSavings = 0;
    sc.cartTotal = 0;

    for (const sci of sc.shoppingCartItemList) {
      if (sci.product) {
        const p = this.getProduct(sci.product.itemId);
        if (p) {
          sci.product = p;
          sci.price = p.price;
        }
      }
      sci.promoSavings = 0;
    }
  }

  dedupeCartItems(cartItems) {
    const quantityMap = new Map();
    for (const sci of cartItems) {
      if (!sci.product) {
        continue;
      }
      const itemId = sci.product.itemId;
      quantityMap.set(itemId, (quantityMap.get(itemId) ?? 0) + sci.quantity);
    }
    const result = [];
    for (const [itemId, quantity] of quantityMap.entries()) {
      const p = this.getProduct(itemId);
      if (!p) {
        continue;
      }
      const newItem = createShoppingCartItem();
      newItem.quantity = quantity;
      newItem.price = p.price;
      newItem.product = p;
      result.push(newItem);
    }
    return result;
  }

  addItem(cartId, itemId, quantity) {
    const cart = this.getShoppingCart(cartId);
    const product = this.getProduct(itemId);
    if (!product) {
      return cart;
    }
    const sci = createShoppingCartItem();
    sci.product = product;
    sci.quantity = quantity;
    sci.price = product.price;
    addShoppingCartItem(cart, sci);
    this.priceShoppingCart(cart);
    cart.shoppingCartItemList = this.dedupeCartItems(cart.shoppingCartItemList);
    this.priceShoppingCart(cart);
    return cart;
  }

  deleteItem(cartId, itemId, quantity) {
    const cart = this.getShoppingCart(cartId);
    const toRemove = [];
    for (const sci of cart.shoppingCartItemList) {
      if (sci.product && itemId === sci.product.itemId) {
        if (quantity >= sci.quantity) {
          toRemove.push(sci);
        } else {
          sci.quantity -= quantity;
        }
      }
    }
    for (const sci of toRemove) {
      removeShoppingCartItem(cart, sci);
    }
    this.priceShoppingCart(cart);
    return cart;
  }
}

module.exports = { ShoppingCartService };
