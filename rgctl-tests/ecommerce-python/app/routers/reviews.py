from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.review import CreateReviewRequest, ReviewResponse
from app.services.review import ReviewService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/products/{product_id}/reviews", tags=["reviews"])


@router.get("", response_model=list[ReviewResponse])
def list_reviews(product_id: str, db: Session = Depends(get_db)) -> list[ReviewResponse]:
    return ReviewService(db).list_for_product(product_id)


@router.post("", response_model=ReviewResponse, status_code=201)
def create_review(
    product_id: str,
    req: CreateReviewRequest,
    user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> ReviewResponse:
    return ReviewService(db).create(user.user_id, product_id, req)
