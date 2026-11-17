use std::sync::Arc;

use sqlx::PgPool;

use crate::Config;

/// Shared application state: cheap to clone (everything is reference-counted).
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Arc<Config>,
    /// Face models; `None` when the model files are missing (face search disabled,
    /// tag/name pickup still works).
    pub faces: Option<Arc<vision::FaceEngine>>,
}
