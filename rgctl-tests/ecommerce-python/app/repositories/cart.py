from sqlalchemy import select
from sqlalchemy.orm import Session, joinedload

from app.models.cart import CartItem
from app.repositories.base import BaseRepository


class CartRepository(BaseRepository[CartItem]):
    def __init__(self, db: Session) -> None:
        super().__init__(db, CartItem)

    def list_for_user(self, user_id: str) -> list[CartItem]:
        stmt = (
            select(CartItem)
            .options(joinedload(CartItem.product))
            .where(CartItem.user_id == user_id)
            .order_by(CartItem.product_id)
        )
        return list(self.db.scalars(stmt).unique().all())

    def get_item(self, user_id: str, product_id: str) -> CartItem | None:
        stmt = (
            select(CartItem)
            .options(joinedload(CartItem.product))
            .where(CartItem.user_id == user_id, CartItem.product_id == product_id)
        )
        return self.db.scalars(stmt).unique().first()

    def clear_for_user(self, user_id: str) -> None:
        items = self.list_for_user(user_id)
        for item in items:
            self.db.delete(item)
        self.db.commit()
