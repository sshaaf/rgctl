use axum::{
    extract::{FromRequestParts, Request},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::{error::AppError, state::AppState, utils::jwt};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: String,
    pub email: String,
    pub role: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth = parts.headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()).ok_or(AppError::Unauthorized)?;
        let token = auth.strip_prefix("Bearer ").ok_or(AppError::Unauthorized)?;
        let claims = jwt::verify(token, &state.config.jwt_secret)?;
        Ok(AuthUser { user_id: claims.sub, email: claims.email, role: claims.role })
    }
}

pub async fn optional_auth(state: AppState, req: Request, next: Next) -> Result<Response, StatusCode> {
    Ok(next.run(req).await)
}
