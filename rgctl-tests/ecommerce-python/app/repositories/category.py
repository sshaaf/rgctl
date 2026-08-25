from sqlalchemy import select
from sqlalchemy.orm import Session

from app.models.category import Category
from app.repositories.base import BaseRepository


class CategoryRepository(BaseRepository[Category]):
    def __init__(self, db: Session) -> None:
        super().__init__(db, Category)

    def find_by_slug(self, slug: str) -> Category | None:
        stmt = select(Category).where(Category.slug == slug)
        return self.db.scalars(stmt).first()

    def list_all(self) -> list[Category]:
        stmt = select(Category).order_by(Category.name)
        return list(self.db.scalars(stmt).all())
