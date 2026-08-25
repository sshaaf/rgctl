"""CoolStore in-memory services: catalog, promo, shipping, cart pricing, orders."""

from __future__ import annotations

import threading
from typing import Optional

from app.coolstore.models import (
    CatalogProduct,
    CoolstoreOrder,
    CoolstoreOrderItem,
    ShoppingCart,
    ShoppingCartItem,
)


class CoolstoreProductService:
    def __init__(self) -> None:
        self._catalog: dict[str, CatalogProduct] = {}
        self._seed("329299", "Red Fedora", "Official Red Hat Fedora", 34.99)
        self._seed("329199", "Forge Laptop Sticker", "JBoss Community sticker", 8.50)
        self._seed("165613", "Solid Performance Polo", "Moisture-wicking polo", 17.80)
        self._seed("165614", "Ogios T-shirt", "CoolStore tee", 11.50)
        self._seed("165954", "Quarkus Stickers", "Pack of stickers", 9.99)

    def _seed(self, item_id: str, name: str, desc: str, price: float) -> None:
        self._catalog[item_id] = CatalogProduct(item_id, name, desc, price)

    def get_products(self) -> list[CatalogProduct]:
        return list(self._catalog.values())

    def get_product_by_item_id(self, item_id: str) -> Optional[CatalogProduct]:
        return self._catalog.get(item_id)

    def as_map(self) -> dict[str, CatalogProduct]:
        return dict(self._catalog)


class PromoService:
    def __init__(self) -> None:
        self._percent_off_by_item: dict[str, float] = {"329299": 0.25}

    def apply_cart_item_promotions(self, shopping_cart: ShoppingCart) -> None:
        if not shopping_cart.shoppingCartItemList:
            return
        for sci in shopping_cart.shoppingCartItemList:
            if sci.product is None:
                continue
            pct = self._percent_off_by_item.get(sci.product.itemId)
            if pct is not None:
                sci.promoSavings = sci.product.price * pct * -1
                sci.price = sci.product.price * (1 - pct)

    def apply_shipping_promotions(self, shopping_cart: ShoppingCart) -> None:
        if shopping_cart.cartItemTotal >= 75:
            shopping_cart.shippingPromoSavings = shopping_cart.shippingTotal * -1
            shopping_cart.shippingTotal = 0


class ShippingService:
    def calculate_shipping(self, sc: ShoppingCart) -> float:
        total = sc.cartItemTotal
        if 0 <= total < 25:
            return 2.99
        if 25 <= total < 50:
            return 4.99
        if 50 <= total < 75:
            return 6.99
        if 75 <= total < 100:
            return 8.99
        if total >= 100:
            return 10.99
        return 0.0

    def calculate_shipping_insurance(self, sc: ShoppingCart) -> float:
        total = sc.cartItemTotal
        if 25 <= total < 100:
            return round(total * 0.02, 2)
        if total >= 100:
            return round(total * 0.015, 2)
        return 0.0


class CoolstoreOrderService:
    def __init__(self) -> None:
        self._seq = 1
        self._lock = threading.Lock()
        self._orders: dict[int, CoolstoreOrder] = {}

    def process(self, cart: ShoppingCart) -> CoolstoreOrder:
        with self._lock:
            order_id = self._seq
            self._seq += 1
        order = CoolstoreOrder(
            orderId=order_id,
            cartId=cart.cartId,
            cartTotal=cart.cartTotal,
            items=[],
        )
        for sci in cart.shoppingCartItemList:
            if sci.product is not None:
                order.items.append(
                    CoolstoreOrderItem(sci.product.itemId, sci.quantity, sci.price)
                )
        self._orders[order.orderId] = order
        return order

    def get_orders(self) -> list[CoolstoreOrder]:
        return list(self._orders.values())

    def get_order_by_id(self, order_id: int) -> Optional[CoolstoreOrder]:
        return self._orders.get(order_id)


class ShoppingCartService:
    def __init__(
        self,
        product_service: CoolstoreProductService,
        promo_service: PromoService,
        shipping_service: ShippingService,
        order_service: CoolstoreOrderService,
    ) -> None:
        self._product_service = product_service
        self._promo_service = promo_service
        self._shipping_service = shipping_service
        self._order_service = order_service
        self._carts: dict[str, ShoppingCart] = {}
        self._lock = threading.Lock()

    def get_shopping_cart(self, cart_id: str) -> ShoppingCart:
        with self._lock:
            if cart_id not in self._carts:
                self._carts[cart_id] = ShoppingCart(cartId=cart_id)
            return self._carts[cart_id]

    def get_product(self, item_id: str) -> Optional[CatalogProduct]:
        return self._product_service.get_product_by_item_id(item_id)

    def check_out_shopping_cart(self, cart_id: str) -> ShoppingCart:
        cart = self.get_shopping_cart(cart_id)
        self.price_shopping_cart(cart)
        self._order_service.process(cart)
        cart.reset_shopping_cart_item_list()
        self.price_shopping_cart(cart)
        return cart

    def price_shopping_cart(self, sc: ShoppingCart | None) -> None:
        """Mutates ShoppingCart totals — primary CPG field-write site."""
        if sc is None:
            return
        self._init_shopping_cart_for_pricing(sc)

        if sc.shoppingCartItemList:
            self._promo_service.apply_cart_item_promotions(sc)

            for sci in sc.shoppingCartItemList:
                sc.cartItemPromoSavings += sci.promoSavings * sci.quantity
                sc.cartItemTotal += sci.price * sci.quantity

            sc.shippingTotal = self._shipping_service.calculate_shipping(sc)
            if sc.cartItemTotal >= 25:
                sc.shippingTotal += self._shipping_service.calculate_shipping_insurance(sc)

        self._promo_service.apply_shipping_promotions(sc)
        sc.cartTotal = sc.cartItemTotal + sc.shippingTotal

    def _init_shopping_cart_for_pricing(self, sc: ShoppingCart) -> None:
        sc.cartItemTotal = 0
        sc.cartItemPromoSavings = 0
        sc.shippingTotal = 0
        sc.shippingPromoSavings = 0
        sc.cartTotal = 0

        for sci in sc.shoppingCartItemList:
            if sci.product is not None:
                p = self.get_product(sci.product.itemId)
                if p is not None:
                    sci.product = p
                    sci.price = p.price
            sci.promoSavings = 0

    def dedupe_cart_items(self, cart_items: list[ShoppingCartItem]) -> list[ShoppingCartItem]:
        quantity_map: dict[str, int] = {}
        for sci in cart_items:
            if sci.product is None:
                continue
            item_id = sci.product.itemId
            quantity_map[item_id] = quantity_map.get(item_id, 0) + sci.quantity

        result: list[ShoppingCartItem] = []
        for item_id, quantity in quantity_map.items():
            p = self.get_product(item_id)
            if p is None:
                continue
            result.append(ShoppingCartItem(quantity=quantity, price=p.price, product=p))
        return result


# Module-level singletons (in-memory store shared across requests)
product_service = CoolstoreProductService()
promo_service = PromoService()
shipping_service = ShippingService()
order_service = CoolstoreOrderService()
shopping_cart_service = ShoppingCartService(
    product_service, promo_service, shipping_service, order_service
)
