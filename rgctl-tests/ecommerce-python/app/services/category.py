import uuid

from sqlalchemy.orm import Session

from app.exceptions import ConflictError, NotFoundError
from app.models.category import Category
from app.repositories.category import CategoryRepository
from app.schemas.category import CategoryResponse, CreateCategoryRequest


class CategoryService:
    def __init__(self, db: Session) -> None:
        self.categories = CategoryRepository(db)

    def list_all(self) -> list[CategoryResponse]:
        return [CategoryResponse.model_validate(c) for c in self.categories.list_all()]

    def get(self, category_id: str) -> CategoryResponse:
        category = self.categories.get(category_id)
        if not category:
            raise NotFoundError("Category not found")
        return CategoryResponse.model_validate(category)

    def create(self, req: CreateCategoryRequest) -> CategoryResponse:
        if self.categories.find_by_slug(req.slug):
            raise ConflictError("slug already exists")
        category = Category(id=str(uuid.uuid4()), name=req.name, slug=req.slug)
        self.categories.add(category)
        return CategoryResponse.model_validate(category)
