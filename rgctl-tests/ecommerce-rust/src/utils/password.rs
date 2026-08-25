use crate::error::{AppError, AppResult};

pub fn hash(password: &str) -> AppResult<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).map_err(|e| AppError::BadRequest(e.to_string()))
}

pub fn verify(password: &str, hash: &str) -> AppResult<bool> {
    bcrypt::verify(password, hash).map_err(|e| AppError::BadRequest(e.to_string()))
}
