from dataclasses import dataclass

from app.exceptions import UnauthorizedError
from app.utils.security import decode_access_token


@dataclass
class AuthUser:
    user_id: str
    email: str
    role: str


def get_auth_user(authorization: str | None = None) -> AuthUser:
    if not authorization:
        raise UnauthorizedError()
    parts = authorization.split(" ", 1)
    if len(parts) != 2 or parts[0].lower() != "bearer":
        raise UnauthorizedError()
    try:
        payload = decode_access_token(parts[1])
    except Exception as exc:
        raise UnauthorizedError() from exc
    user_id = payload.get("sub")
    email = payload.get("email")
    role = payload.get("role")
    if not user_id or not email or not role:
        raise UnauthorizedError()
    return AuthUser(user_id=user_id, email=email, role=role)
