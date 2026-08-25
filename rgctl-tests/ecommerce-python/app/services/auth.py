import uuid

from sqlalchemy.orm import Session

from app.exceptions import ConflictError, NotFoundError, UnauthorizedError
from app.models.user import User
from app.repositories.user import UserRepository
from app.schemas.auth import AuthResponse, LoginRequest, RegisterRequest
from app.utils.security import create_access_token, hash_password, verify_password


class AuthService:
    def __init__(self, db: Session) -> None:
        self.users = UserRepository(db)

    def register(self, req: RegisterRequest) -> AuthResponse:
        if self.users.find_by_email(req.email):
            raise ConflictError("email already registered")
        user = User(
            id=str(uuid.uuid4()),
            email=req.email,
            password_hash=hash_password(req.password),
            name=req.name,
            role="customer",
        )
        self.users.add(user)
        token = create_access_token(user.id, user.email, user.role)
        return AuthResponse(token=token, user_id=user.id, email=user.email, name=user.name)

    def login(self, req: LoginRequest) -> AuthResponse:
        user = self.users.find_by_email(req.email)
        if not user or not verify_password(req.password, user.password_hash):
            raise UnauthorizedError()
        token = create_access_token(user.id, user.email, user.role)
        return AuthResponse(token=token, user_id=user.id, email=user.email, name=user.name)
