from sqlalchemy import select
from sqlalchemy.orm import Session

from app.models.review import Review
from app.repositories.base import BaseRepository


class ReviewRepository(BaseRepository[Review]):
    def __init__(self, db: Session) -> None:
        super().__init__(db, Review)

    def list_for_product(self, product_id: str) -> list[Review]:
        stmt = (
            select(Review)
            .where(Review.product_id == product_id)
            .order_by(Review.created_at.desc())
        )
        return list(self.db.scalars(stmt).all())

    def find_by_user_and_product(self, user_id: str, product_id: str) -> Review | None:
        stmt = select(Review).where(Review.user_id == user_id, Review.product_id == product_id)
        return self.db.scalars(stmt).first()
