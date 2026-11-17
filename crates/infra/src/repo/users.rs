use domain::{dto::CreateUserRequest, models::User};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppResult, auth, error::map_unique};

const USER_COLS: &str = "id, email, full_name, role, is_active, created_at";

#[derive(sqlx::FromRow)]
pub struct UserWithHash {
    #[sqlx(flatten)]
    pub user: User,
    pub password_hash: String,
}

pub async fn count(db: &PgPool) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM users").fetch_one(db).await?)
}

pub async fn create(db: &PgPool, req: &CreateUserRequest) -> AppResult<User> {
    let hash = auth::hash_password(req.password.clone()).await?;
    sqlx::query_as(&format!(
        "INSERT INTO users (id, email, password_hash, full_name, role)
         VALUES ($1, $2, $3, $4, $5) RETURNING {USER_COLS}"
    ))
    .bind(Uuid::new_v4())
    .bind(req.email.trim().to_lowercase())
    .bind(hash)
    .bind(req.full_name.trim())
    .bind(req.role)
    .fetch_one(db)
    .await
    .map_err(|e| map_unique(e, "user with this email"))
}

pub async fn find_with_hash_by_email(db: &PgPool, email: &str) -> AppResult<Option<UserWithHash>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {USER_COLS}, password_hash FROM users WHERE lower(email) = lower($1)"
    ))
    .bind(email.trim())
    .fetch_optional(db)
    .await?)
}

pub async fn find_by_id(db: &PgPool, id: Uuid) -> AppResult<Option<User>> {
    Ok(sqlx::query_as(&format!("SELECT {USER_COLS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(db)
        .await?)
}
