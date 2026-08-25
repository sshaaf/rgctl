import uuid

from sqlalchemy.orm import Session

from app.exceptions import ConflictError, NotFoundError
from app.models.product import Product
from app.repositories.category import CategoryRepository
from app.repositories.product import ProductRepository
from app.schemas.product import CreateProductRequest, ProductResponse


class ProductService:
    def __init__(self, db: Session) -> None:
        self.products = ProductRepository(db)
        self.categories = CategoryRepository(db)

    def list_all(self) -> list[ProductResponse]:
        return [ProductResponse.model_validate(p) for p in self.products.list_all()]

    def get(self, product_id: str) -> ProductResponse:
        product = self.products.get(product_id)
        if not product:
            raise NotFoundError("Product not found")
        return ProductResponse.model_validate(product)

    def create(self, req: CreateProductRequest) -> ProductResponse:
        if not self.categories.get(req.category_id):
            raise NotFoundError("Category not found")
        if self.products.find_by_slug(req.slug):
            raise ConflictError("slug already exists")
        product = Product(
            id=str(uuid.uuid4()),
            category_id=req.category_id,
            name=req.name,
            slug=req.slug,
            description=req.description,
            price_cents=req.price_cents,
            stock=req.stock,
        )
        self.products.add(product)
        return ProductResponse.model_validate(product)
