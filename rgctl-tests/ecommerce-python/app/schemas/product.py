from datetime import datetime

from pydantic import BaseModel, Field

from app.schemas.common import ORMModel


class CreateProductRequest(BaseModel):
    category_id: str
    name: str = Field(min_length=1, max_length=255)
    slug: str = Field(min_length=1, max_length=255)
    description: str = ""
    price_cents: int = Field(ge=0)
    stock: int = Field(ge=0, default=0)


class ProductResponse(ORMModel):
    id: str
    category_id: str
    name: str
    slug: str
    description: str
    price_cents: int
    stock: int
    created_at: datetime
