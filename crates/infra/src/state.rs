use std::sync::Arc;

use sqlx::PgPool;

use crate::Config;

/// Shared application state: cheap to clone (pool and config are reference-counted).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
}
