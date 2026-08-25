from fastapi import APIRouter, HTTPException
from pydantic import BaseModel, ConfigDict

from app.coolstore import services as svc
from app.coolstore.models import (
    CatalogProduct,
    CoolstoreOrder,
    ShoppingCart,
    ShoppingCartItem,
)


class CatalogProductOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    itemId: str
    name: str
    desc: str
    price: float


class ShoppingCartItemOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    price: float
    quantity: int
    promoSavings: float
    product: CatalogProductOut | None = None


class ShoppingCartOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    cartId: str
    cartItemTotal: float
    cartItemPromoSavings: float
    shippingTotal: float
    shippingPromoSavings: float
    cartTotal: float
    shoppingCartItemList: list[ShoppingCartItemOut]


class CoolstoreOrderItemOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    productId: str
    quantity: int
    price: float


class CoolstoreOrderOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    orderId: int
    cartId: str
    cartTotal: float
    items: list[CoolstoreOrderItemOut]


def _cart_out(cart: ShoppingCart) -> ShoppingCartOut:
    return ShoppingCartOut.model_validate(cart)


def _order_out(order: CoolstoreOrder) -> CoolstoreOrderOut:
    return CoolstoreOrderOut.model_validate(order)


products_router = APIRouter(prefix="/services/products", tags=["coolstore-products"])
cart_router = APIRouter(prefix="/services/cart", tags=["coolstore-cart"])
orders_router = APIRouter(prefix="/services/orders", tags=["coolstore-orders"])


@products_router.get("", response_model=list[CatalogProductOut])
def list_products() -> list[CatalogProduct]:
    return svc.product_service.get_products()


@products_router.get("/{item_id}", response_model=CatalogProductOut | None)
def get_product(item_id: str) -> CatalogProduct | None:
    return svc.product_service.get_product_by_item_id(item_id)


@cart_router.get("/{cart_id}", response_model=ShoppingCartOut)
def get_cart(cart_id: str) -> ShoppingCartOut:
    return _cart_out(svc.shopping_cart_service.get_shopping_cart(cart_id))


@cart_router.post("/checkout/{cart_id}", response_model=ShoppingCartOut)
def checkout(cart_id: str) -> ShoppingCartOut:
    return _cart_out(svc.shopping_cart_service.check_out_shopping_cart(cart_id))


@cart_router.post("/{cart_id}/{item_id}/{quantity}", response_model=ShoppingCartOut)
def add_item(cart_id: str, item_id: str, quantity: int) -> ShoppingCartOut:
    cart = svc.shopping_cart_service.get_shopping_cart(cart_id)
    product = svc.shopping_cart_service.get_product(item_id)
    if product is None:
        return _cart_out(cart)
    sci = ShoppingCartItem(product=product, quantity=quantity, price=product.price)
    cart.add_shopping_cart_item(sci)
    svc.shopping_cart_service.price_shopping_cart(cart)
    cart.shoppingCartItemList = svc.shopping_cart_service.dedupe_cart_items(
        cart.shoppingCartItemList
    )
    svc.shopping_cart_service.price_shopping_cart(cart)
    return _cart_out(cart)


@cart_router.delete("/{cart_id}/{item_id}/{quantity}", response_model=ShoppingCartOut)
def delete_item(cart_id: str, item_id: str, quantity: int) -> ShoppingCartOut:
    cart = svc.shopping_cart_service.get_shopping_cart(cart_id)
    to_remove: list[ShoppingCartItem] = []
    for sci in cart.shoppingCartItemList:
        if sci.product is not None and item_id == sci.product.itemId:
            if quantity >= sci.quantity:
                to_remove.append(sci)
            else:
                sci.quantity -= quantity
    for sci in to_remove:
        cart.remove_shopping_cart_item(sci)
    svc.shopping_cart_service.price_shopping_cart(cart)
    return _cart_out(cart)


@orders_router.get("", response_model=list[CoolstoreOrderOut])
def list_orders() -> list[CoolstoreOrderOut]:
    return [_order_out(o) for o in svc.order_service.get_orders()]


@orders_router.get("/{order_id}", response_model=CoolstoreOrderOut)
def get_order(order_id: int) -> CoolstoreOrderOut:
    order = svc.order_service.get_order_by_id(order_id)
    if order is None:
        raise HTTPException(status_code=404, detail="Order not found")
    return _order_out(order)
