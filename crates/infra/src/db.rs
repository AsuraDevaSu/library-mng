use std::time::Duration;

use domain::{dto::CreateUserRequest, models::Role};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{AppResult, Config, repo};

pub async fn connect(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
}

/// Applies `migrations/` (embedded into the binary at compile time).
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

/// Creates the first admin from ADMIN_EMAIL / ADMIN_PASSWORD when there are no users yet.
pub async fn bootstrap_admin(pool: &PgPool, config: &Config) -> AppResult<()> {
    let Some((email, password)) = &config.bootstrap_admin else {
        return Ok(());
    };
    if repo::users::count(pool).await? > 0 {
        return Ok(());
    }
    let req = CreateUserRequest {
        email: email.clone(),
        password: password.clone(),
        full_name: "Administrator".into(),
        role: Role::Admin,
    };
    repo::users::create(pool, &req).await?;
    tracing::info!(%email, "bootstrap admin created");
    Ok(())
}
