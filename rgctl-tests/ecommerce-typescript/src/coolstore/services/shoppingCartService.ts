import type { CatalogProduct } from '../models/catalogProduct';
import {
  addShoppingCartItem,
  createShoppingCart,
  removeShoppingCartItem,
  resetShoppingCartItemList,
  type ShoppingCart,
} from '../models/shoppingCart';
import { createShoppingCartItem, type ShoppingCartItem } from '../models/shoppingCartItem';
import type { CoolstoreOrderService } from './coolstoreOrderService';
import type { CoolstoreProductService } from './coolstoreProductService';
import type { PromoService } from './promoService';
import type { ShippingService } from './shippingService';

export class ShoppingCartService {
  private readonly carts = new Map<string, ShoppingCart>();

  constructor(
    private readonly productService: CoolstoreProductService,
    private readonly promoService: PromoService,
    private readonly shippingService: ShippingService,
    private readonly orderService: CoolstoreOrderService,
  ) {}

  getShoppingCart(cartId: string): ShoppingCart {
    let cart = this.carts.get(cartId);
    if (!cart) {
      cart = createShoppingCart(cartId);
      this.carts.set(cartId, cart);
    }
    return cart;
  }

  getProduct(itemId: string): CatalogProduct | undefined {
    return this.productService.getProductByItemId(itemId);
  }

  checkOutShoppingCart(cartId: string): ShoppingCart {
    const cart = this.getShoppingCart(cartId);
    this.priceShoppingCart(cart);
    this.orderService.process(cart);
    resetShoppingCartItemList(cart);
    this.priceShoppingCart(cart);
    return cart;
  }

  /** Mutates ShoppingCart totals — primary CPG field-write site. */
  priceShoppingCart(sc: ShoppingCart | null | undefined): void {
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

  private initShoppingCartForPricing(sc: ShoppingCart): void {
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

  dedupeCartItems(cartItems: ShoppingCartItem[]): ShoppingCartItem[] {
    const quantityMap = new Map<string, number>();
    for (const sci of cartItems) {
      if (!sci.product) {
        continue;
      }
      const itemId = sci.product.itemId;
      quantityMap.set(itemId, (quantityMap.get(itemId) ?? 0) + sci.quantity);
    }
    const result: ShoppingCartItem[] = [];
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

  addItem(cartId: string, itemId: string, quantity: number): ShoppingCart {
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

  deleteItem(cartId: string, itemId: string, quantity: number): ShoppingCart {
    const cart = this.getShoppingCart(cartId);
    const toRemove: ShoppingCartItem[] = [];
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
