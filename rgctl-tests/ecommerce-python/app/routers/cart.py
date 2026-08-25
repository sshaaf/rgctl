from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.cart import AddCartItemRequest, CartItemResponse
from app.services.cart import CartService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/cart", tags=["cart"])


@router.get("", response_model=list[CartItemResponse])
def list_cart(user: AuthUser = Depends(require_auth), db: Session = Depends(get_db)) -> list[CartItemResponse]:
    return CartService(db).list_items(user.user_id)


@router.post("/items", response_model=CartItemResponse, status_code=201)
def add_cart_item(
    req: AddCartItemRequest,
    user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> CartItemResponse:
    return CartService(db).add_item(user.user_id, req)


@router.delete("/items/{product_id}", status_code=204)
def remove_cart_item(
    product_id: str,
    user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> None:
    CartService(db).remove_item(user.user_id, product_id)
