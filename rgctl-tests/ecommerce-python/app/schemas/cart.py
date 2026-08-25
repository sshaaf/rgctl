from pydantic import BaseModel, Field

from app.schemas.common import ORMModel
from app.schemas.product import ProductResponse


class AddCartItemRequest(BaseModel):
    product_id: str
    quantity: int = Field(ge=1, default=1)


class CartItemResponse(ORMModel):
    product_id: str
    quantity: int
    product: ProductResponse
