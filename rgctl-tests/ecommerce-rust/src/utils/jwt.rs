use crate::error::AppResult;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
}

pub fn sign(user_id: &str, email: &str, role: &str, secret: &str) -> AppResult<String> {
    let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;
    let claims = Claims { sub: user_id.into(), email: email.into(), role: role.into(), exp };
    Ok(encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))?)
}

pub fn verify(token: &str, secret: &str) -> AppResult<Claims> {
    Ok(decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())?.claims)
}
