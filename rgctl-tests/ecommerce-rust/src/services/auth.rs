use uuid::Uuid;
use crate::{
    dto::auth::{AuthResponse, LoginRequest, RegisterRequest},
    error::{AppError, AppResult},
    models::user::User,
    repositories::user as user_repo,
    state::AppState,
    utils::{jwt, password, time},
};

pub async fn register(state: &AppState, req: RegisterRequest) -> AppResult<AuthResponse> {
    if user_repo::find_by_email(&state.pool, &req.email).await?.is_some() {
        return Err(AppError::Conflict("email already registered".into()));
    }
    let user = User {
        id: Uuid::new_v4().to_string(),
        email: req.email.clone(),
        password_hash: password::hash(&req.password)?,
        name: req.name.clone(),
        role: "customer".into(),
        created_at: time::now_iso(),
    };
    user_repo::create(&state.pool, &user).await?;
    let token = jwt::sign(&user.id, &user.email, &user.role, &state.config.jwt_secret)?;
    Ok(AuthResponse { token, user_id: user.id, email: user.email, name: user.name })
}

pub async fn login(state: &AppState, req: LoginRequest) -> AppResult<AuthResponse> {
    let user = user_repo::find_by_email(&state.pool, &req.email).await?.ok_or(AppError::Unauthorized)?;
    if !password::verify(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }
    let token = jwt::sign(&user.id, &user.email, &user.role, &state.config.jwt_secret)?;
    Ok(AuthResponse { token, user_id: user.id, email: user.email, name: user.name })
}
