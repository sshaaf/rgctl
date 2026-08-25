from sqlalchemy import ForeignKey, Integer, String
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.database import Base


class CartItem(Base):
    __tablename__ = "cart_items"

    user_id: Mapped[str] = mapped_column(String(36), ForeignKey("users.id"), primary_key=True)
    product_id: Mapped[str] = mapped_column(String(36), ForeignKey("products.id"), primary_key=True)
    quantity: Mapped[int] = mapped_column(Integer)

    user = relationship("User", back_populates="cart_items")
    product = relationship("Product", back_populates="cart_items")
