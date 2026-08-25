from datetime import datetime

from pydantic import BaseModel

from app.schemas.common import ORMModel
from app.schemas.product import ProductResponse


class OrderItemResponse(ORMModel):
    id: str
    product_id: str
    quantity: int
    unit_price_cents: int
    product: ProductResponse | None = None


class OrderResponse(ORMModel):
    id: str
    user_id: str
    status: str
    total_cents: int
    created_at: datetime
    items: list[OrderItemResponse] = []
