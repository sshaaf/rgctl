import uuid

from sqlalchemy.orm import Session

from app.exceptions import BadRequestError, NotFoundError
from app.models.order import Order, OrderItem
from app.repositories.cart import CartRepository
from app.repositories.order import OrderRepository
from app.repositories.product import ProductRepository
from app.schemas.order import OrderResponse


class OrderService:
    def __init__(self, db: Session) -> None:
        self.orders = OrderRepository(db)
        self.cart = CartRepository(db)
        self.products = ProductRepository(db)

    def list_orders(self, user_id: str) -> list[OrderResponse]:
        orders = self.orders.list_for_user(user_id)
        return [OrderResponse.model_validate(o) for o in orders]

    def get_order(self, user_id: str, order_id: str) -> OrderResponse:
        order = self.orders.get_for_user(order_id, user_id)
        if not order:
            raise NotFoundError("Order not found")
        return OrderResponse.model_validate(order)

    def checkout(self, user_id: str) -> OrderResponse:
        cart_items = self.cart.list_for_user(user_id)
        if not cart_items:
            raise BadRequestError("Cart is empty")

        total_cents = 0
        for item in cart_items:
            if item.product.stock < item.quantity:
                raise BadRequestError(f"Insufficient stock for {item.product.name}")
            total_cents += item.product.price_cents * item.quantity

        order = Order(
            id=str(uuid.uuid4()),
            user_id=user_id,
            status="confirmed",
            total_cents=total_cents,
        )
        self.orders.add(order)

        for item in cart_items:
            order_item = OrderItem(
                id=str(uuid.uuid4()),
                order_id=order.id,
                product_id=item.product_id,
                quantity=item.quantity,
                unit_price_cents=item.product.price_cents,
            )
            self.orders.add_item(order_item)
            self.products.decrement_stock(item.product, item.quantity)

        self.cart.clear_for_user(user_id)

        loaded = self.orders.get_for_user(order.id, user_id)
        assert loaded is not None
        return OrderResponse.model_validate(loaded)
