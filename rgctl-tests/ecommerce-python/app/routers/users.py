from fastapi import APIRouter, Depends
from sqlalchemy.orm import Session

from app.database import get_db
from app.middleware.auth import require_auth
from app.schemas.user import UserResponse, UserUpdateRequest
from app.services.user import UserService
from app.utils.dependencies import AuthUser

router = APIRouter(prefix="/api/users", tags=["users"])


@router.get("/me", response_model=UserResponse)
def get_me(user: AuthUser = Depends(require_auth), db: Session = Depends(get_db)) -> UserResponse:
    return UserService(db).get(user.user_id)


@router.patch("/me", response_model=UserResponse)
def update_me(
    req: UserUpdateRequest,
    user: AuthUser = Depends(require_auth),
    db: Session = Depends(get_db),
) -> UserResponse:
    return UserService(db).update(user.user_id, req)
