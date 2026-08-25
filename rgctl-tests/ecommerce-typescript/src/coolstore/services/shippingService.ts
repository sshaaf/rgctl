import type { ShoppingCart } from '../models/shoppingCart';

function round2(v: number): number {
  return Math.round(v * 100) / 100;
}

export class ShippingService {
  calculateShipping(sc: ShoppingCart): number {
    const total = sc.cartItemTotal;
    if (total >= 0 && total < 25) return 2.99;
    if (total >= 25 && total < 50) return 4.99;
    if (total >= 50 && total < 75) return 6.99;
    if (total >= 75 && total < 100) return 8.99;
    if (total >= 100) return 10.99;
    return 0;
  }

  calculateShippingInsurance(sc: ShoppingCart): number {
    const total = sc.cartItemTotal;
    if (total >= 25 && total < 100) return round2(total * 0.02);
    if (total >= 100) return round2(total * 0.015);
    return 0;
  }
}
