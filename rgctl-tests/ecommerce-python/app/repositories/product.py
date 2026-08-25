from sqlalchemy import select
from sqlalchemy.orm import Session, joinedload

from app.models.product import Product
from app.repositories.base import BaseRepository


class ProductRepository(BaseRepository[Product]):
    def __init__(self, db: Session) -> None:
        super().__init__(db, Product)

    def find_by_slug(self, slug: str) -> Product | None:
        stmt = select(Product).where(Product.slug == slug)
        return self.db.scalars(stmt).first()

    def list_all(self) -> list[Product]:
        stmt = select(Product).order_by(Product.name)
        return list(self.db.scalars(stmt).all())

    def get_with_category(self, product_id: str) -> Product | None:
        stmt = select(Product).options(joinedload(Product.category)).where(Product.id == product_id)
        return self.db.scalars(stmt).first()

    def decrement_stock(self, product: Product, quantity: int) -> Product:
        product.stock -= quantity
        self.db.commit()
        self.db.refresh(product)
        return product
