from sqlalchemy.orm import Session

from app.exceptions import BadRequestError, NotFoundError
from app.models.cart import CartItem
from app.repositories.cart import CartRepository
from app.repositories.product import ProductRepository
from app.schemas.cart import AddCartItemRequest, CartItemResponse


class CartService:
    def __init__(self, db: Session) -> None:
        self.cart = CartRepository(db)
        self.products = ProductRepository(db)

    def list_items(self, user_id: str) -> list[CartItemResponse]:
        items = self.cart.list_for_user(user_id)
        return [CartItemResponse.model_validate(item) for item in items]

    def add_item(self, user_id: str, req: AddCartItemRequest) -> CartItemResponse:
        product = self.products.get(req.product_id)
        if not product:
            raise NotFoundError("Product not found")
        if product.stock < req.quantity:
            raise BadRequestError("Insufficient stock")

        existing = self.cart.get_item(user_id, req.product_id)
        if existing:
            existing.quantity += req.quantity
            self.cart.db.commit()
            self.cart.db.refresh(existing)
            item = self.cart.get_item(user_id, req.product_id)
            assert item is not None
            return CartItemResponse.model_validate(item)

        item = CartItem(user_id=user_id, product_id=req.product_id, quantity=req.quantity)
        self.cart.add(item)
        loaded = self.cart.get_item(user_id, req.product_id)
        assert loaded is not None
        return CartItemResponse.model_validate(loaded)

    def remove_item(self, user_id: str, product_id: str) -> None:
        item = self.cart.get_item(user_id, product_id)
        if not item:
            raise NotFoundError("Cart item not found")
        self.cart.delete(item)
