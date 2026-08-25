from datetime import datetime

from pydantic import BaseModel, Field

from app.schemas.common import ORMModel


class CreateCategoryRequest(BaseModel):
    name: str = Field(min_length=1, max_length=255)
    slug: str = Field(min_length=1, max_length=255)


class CategoryResponse(ORMModel):
    id: str
    name: str
    slug: str
    created_at: datetime
