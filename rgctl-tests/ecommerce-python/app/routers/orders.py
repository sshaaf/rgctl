from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.order import OrderResponse
from app.services.order import OrderService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/orders", tags=["orders"])


@router.get("", response_model=list[OrderResponse])
def list_orders(user: AuthUser = Depends(require_auth), db: Session = Depends(get_db)) -> list[OrderResponse]:
    return OrderService(db).list_orders(user.user_id)


@router.post("", response_model=OrderResponse, status_code=201)
def checkout(user: AuthUser = Depends(require_auth), db: Session = Depends(get_db)) -> OrderResponse:
    return OrderService(db).checkout(user.user_id)


@router.get("/{order_id}", response_model=OrderResponse)
def get_order(
    order_id: str,
    user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> OrderResponse:
    return OrderService(db).get_order(user.user_id, order_id)
