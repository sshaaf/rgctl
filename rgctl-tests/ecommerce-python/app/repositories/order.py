from sqlalchemy import select
from sqlalchemy.orm import Session, joinedload

from app.models.order import Order, OrderItem
from app.repositories.base import BaseRepository


class OrderRepository(BaseRepository[Order]):
    def __init__(self, db: Session) -> None:
        super().__init__(db, Order)

    def list_for_user(self, user_id: str) -> list[Order]:
        stmt = (
            select(Order)
            .options(joinedload(Order.items).joinedload(OrderItem.product))
            .where(Order.user_id == user_id)
            .order_by(Order.created_at.desc())
        )
        return list(self.db.scalars(stmt).unique().all())

    def get_for_user(self, order_id: str, user_id: str) -> Order | None:
        stmt = (
            select(Order)
            .options(joinedload(Order.items).joinedload(OrderItem.product))
            .where(Order.id == order_id, Order.user_id == user_id)
        )
        return self.db.scalars(stmt).unique().first()

    def add_item(self, item: OrderItem) -> OrderItem:
        self.db.add(item)
        self.db.commit()
        self.db.refresh(item)
        return item
