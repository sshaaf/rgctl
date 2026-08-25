import uuid

from sqlalchemy import func, select
from sqlalchemy.orm import Session

from app.models.category import Category
from app.models.product import Product


def seed_demo_data(db: Session) -> None:
    count = db.scalar(select(func.count()).select_from(Category))
    if count and count > 0:
        return

    category = Category(
        id=str(uuid.uuid4()),
        name="Electronics",
        slug="electronics",
    )
    db.add(category)
    db.flush()

    products = [
        Product(
            id=str(uuid.uuid4()),
            category_id=category.id,
            name="Wireless Headphones",
            slug="wireless-headphones",
            description="Noise cancelling over-ear headphones",
            price_cents=12999,
            stock=50,
        ),
        Product(
            id=str(uuid.uuid4()),
            category_id=category.id,
            name="USB-C Hub",
            slug="usb-c-hub",
            description="7-in-1 adapter",
            price_cents=4999,
            stock=120,
        ),
    ]
    db.add_all(products)
    db.commit()
