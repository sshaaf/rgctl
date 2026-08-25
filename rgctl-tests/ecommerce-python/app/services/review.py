import uuid

from sqlalchemy.orm import Session

from app.exceptions import ConflictError, NotFoundError
from app.models.review import Review
from app.repositories.product import ProductRepository
from app.repositories.review import ReviewRepository
from app.schemas.review import CreateReviewRequest, ReviewResponse


class ReviewService:
    def __init__(self, db: Session) -> None:
        self.reviews = ReviewRepository(db)
        self.products = ProductRepository(db)

    def list_for_product(self, product_id: str) -> list[ReviewResponse]:
        if not self.products.get(product_id):
            raise NotFoundError("Product not found")
        reviews = self.reviews.list_for_product(product_id)
        return [ReviewResponse.model_validate(r) for r in reviews]

    def create(self, user_id: str, product_id: str, req: CreateReviewRequest) -> ReviewResponse:
        if not self.products.get(product_id):
            raise NotFoundError("Product not found")
        if self.reviews.find_by_user_and_product(user_id, product_id):
            raise ConflictError("You already reviewed this product")
        review = Review(
            id=str(uuid.uuid4()),
            product_id=product_id,
            user_id=user_id,
            rating=req.rating,
            comment=req.comment,
        )
        self.reviews.add(review)
        return ReviewResponse.model_validate(review)
