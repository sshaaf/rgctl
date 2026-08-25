from fastapi import Header

from app.utils.dependencies import AuthUser, get_auth_user


def require_auth(authorization: str | None = Header(default=None)) -> AuthUser:
    return get_auth_user(authorization)
