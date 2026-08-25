from sqlalchemy.orm import Session

from app.exceptions import NotFoundError
from app.repositories.user import UserRepository
from app.schemas.user import UserResponse, UserUpdateRequest


class UserService:
    def __init__(self, db: Session) -> None:
        self.users = UserRepository(db)

    def get(self, user_id: str) -> UserResponse:
        user = self.users.get(user_id)
        if not user:
            raise NotFoundError("User not found")
        return UserResponse.model_validate(user)

    def update(self, user_id: str, req: UserUpdateRequest) -> UserResponse:
        user = self.users.get(user_id)
        if not user:
            raise NotFoundError("User not found")
        if req.name is not None:
            user.name = req.name
            self.users.db.commit()
            self.users.db.refresh(user)
        return UserResponse.model_validate(user)
