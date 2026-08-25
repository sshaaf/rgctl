from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.category import CategoryResponse, CreateCategoryRequest
from app.services.category import CategoryService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/categories", tags=["categories"])


@router.get("", response_model=list[CategoryResponse])
def list_categories(db: Session = Depends(get_db)) -> list[CategoryResponse]:
    return CategoryService(db).list_all()


@router.get("/{category_id}", response_model=CategoryResponse)
def get_category(category_id: str, db: Session = Depends(get_db)) -> CategoryResponse:
    return CategoryService(db).get(category_id)


@router.post("", response_model=CategoryResponse, status_code=201)
def create_category(
    req: CreateCategoryRequest,
    _user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> CategoryResponse:
    return CategoryService(db).create(req)
