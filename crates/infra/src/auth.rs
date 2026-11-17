use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::Utc;
use domain::models::Role;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub role: Role,
    pub exp: i64,
    pub iat: i64,
}

/// Argon2 is deliberately slow: run it off the async executor.
pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| AppError::Internal(e.to_string()))
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub async fn verify_password(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&hash).map_err(|e| AppError::Internal(e.to_string()))?;
        Ok(Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
}

pub fn issue_token(user_id: Uuid, role: Role, secret: &str, ttl_minutes: i64) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims { sub: user_id, role, iat: now, exp: now + ttl_minutes * 60 };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::Internal(e.to_string()))
}

pub fn decode_token(token: &str, secret: &str) -> AppResult<Claims> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|d| d.claims)
        .map_err(|_| AppError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "test-secret-test-secret-test-secret!!";

    #[tokio::test]
    async fn password_roundtrip() {
        let h = hash_password("correct horse".into()).await.unwrap();
        assert!(verify_password("correct horse".into(), h.clone()).await.unwrap());
        assert!(!verify_password("wrong".into(), h).await.unwrap());
    }

    #[test]
    fn token_roundtrip() {
        let id = Uuid::new_v4();
        let t = issue_token(id, Role::Librarian, SECRET, 5).unwrap();
        let c = decode_token(&t, SECRET).unwrap();
        assert_eq!(c.sub, id);
        assert_eq!(c.role, Role::Librarian);
        assert!(decode_token(&t, "another-secret-another-secret-12345").is_err());
    }
}
