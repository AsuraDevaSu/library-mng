use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub db_max_connections: u32,
    pub jwt_secret: String,
    pub jwt_ttl_minutes: i64,
    /// Adds `Secure` to the session cookie. Set to true behind HTTPS.
    pub cookie_secure: bool,
    /// Comma-separated origins allowed by CORS (e.g. the Flutter web dev server).
    pub cors_origins: Vec<String>,
    /// If set and the users table is empty, an admin is created at startup.
    pub bootstrap_admin: Option<(String, String)>,
    /// Where bag-desk photos are stored (never inside the public site root).
    pub bag_photo_dir: std::path::PathBuf,
    /// Folder with the YuNet / SFace ONNX models (see scripts/fetch-models.sh).
    pub models_dir: std::path::PathBuf,
}

impl Config {
    /// Reads configuration from the environment (call `dotenvy::dotenv()` first).
    pub fn from_env() -> Result<Self, String> {
        let required = |k: &str| env::var(k).map_err(|_| format!("missing env var {k}"));
        let jwt_secret = required("JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".into());
        }
        Ok(Self {
            database_url: required("DATABASE_URL")?,
            db_max_connections: env::var("DB_MAX_CONNECTIONS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            jwt_secret,
            jwt_ttl_minutes: env::var("JWT_TTL_MINUTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            cookie_secure: env::var("COOKIE_SECURE").is_ok_and(|v| v == "true" || v == "1"),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            bag_photo_dir: env::var("BAG_PHOTO_DIR").unwrap_or_else(|_| "data/bag-photos".into()).into(),
            models_dir: env::var("MODELS_DIR").unwrap_or_else(|_| "models".into()).into(),
            bootstrap_admin: match (env::var("ADMIN_EMAIL"), env::var("ADMIN_PASSWORD")) {
                (Ok(e), Ok(p)) if !e.is_empty() && !p.is_empty() => Some((e, p)),
                _ => None,
            },
        })
    }
}
