from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.product import CreateProductRequest, ProductResponse
from app.services.product import ProductService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/products", tags=["products"])


@router.get("", response_model=list[ProductResponse])
def list_products(db: Session = Depends(get_db)) -> list[ProductResponse]:
    return ProductService(db).list_all()


@router.get("/{product_id}", response_model=ProductResponse)
def get_product(product_id: str, db: Session = Depends(get_db)) -> ProductResponse:
    return ProductService(db).get(product_id)


@router.post("", response_model=ProductResponse, status_code=201)
def create_product(
    req: CreateProductRequest,
    _user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> ProductResponse:
    return ProductService(db).create(req)
