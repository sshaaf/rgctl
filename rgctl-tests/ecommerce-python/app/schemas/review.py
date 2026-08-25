from datetime import datetime

from pydantic import BaseModel, Field

from app.schemas.common import ORMModel


class CreateReviewRequest(BaseModel):
    rating: int = Field(ge=1, le=5)
    comment: str = ""


class ReviewResponse(ORMModel):
    id: str
    product_id: str
    user_id: str
    rating: int
    comment: str
    created_at: datetime
