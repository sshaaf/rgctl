"""CoolStore dual-API models (mutable cart totals for CPG field-write targets)."""

from __future__ import annotations

from dataclasses import dataclass, field


@dataclass
class CatalogProduct:
    itemId: str
    name: str
    desc: str
    price: float


@dataclass
class ShoppingCartItem:
    price: float = 0.0
    quantity: int = 0
    promoSavings: float = 0.0
    product: CatalogProduct | None = None


@dataclass
class ShoppingCart:
    cartId: str = ""
    cartItemTotal: float = 0.0
    cartItemPromoSavings: float = 0.0
    shippingTotal: float = 0.0
    shippingPromoSavings: float = 0.0
    cartTotal: float = 0.0
    shoppingCartItemList: list[ShoppingCartItem] = field(default_factory=list)

    def reset_shopping_cart_item_list(self) -> None:
        self.shoppingCartItemList = []

    def add_shopping_cart_item(self, sci: ShoppingCartItem | None) -> None:
        if sci is not None:
            self.shoppingCartItemList.append(sci)

    def remove_shopping_cart_item(self, sci: ShoppingCartItem | None) -> bool:
        if sci is None:
            return False
        try:
            self.shoppingCartItemList.remove(sci)
            return True
        except ValueError:
            return False


@dataclass
class CoolstoreOrderItem:
    productId: str
    quantity: int
    price: float


@dataclass
class CoolstoreOrder:
    orderId: int = 0
    cartId: str = ""
    cartTotal: float = 0.0
    items: list[CoolstoreOrderItem] = field(default_factory=list)
