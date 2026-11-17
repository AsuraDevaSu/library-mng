//! Login and token → user resolution, shared by the REST API (bearer header)
//! and the Leptos UI (HttpOnly cookie).

use domain::models::User;

use crate::{AppError, AppResult, AppState, auth, repo::users};

pub const COOKIE_NAME: &str = "lib_session";

pub async fn login(state: &AppState, email: &str, password: &str) -> AppResult<(User, String)> {
    let found = users::find_with_hash_by_email(&state.db, email).await?;
    let Some(found) = found else {
        // Spend comparable time so response timing doesn't reveal which emails exist.
        let _ = auth::hash_password(password.to_owned()).await;
        return Err(AppError::Unauthorized);
    };
    if !auth::verify_password(password.to_owned(), found.password_hash).await? || !found.user.is_active {
        return Err(AppError::Unauthorized);
    }
    let token = auth::issue_token(
        found.user.id,
        found.user.role,
        &state.config.jwt_secret,
        state.config.jwt_ttl_minutes,
    )?;
    Ok((found.user, token))
}

/// Validates the token, then loads the user from the database so that a
/// disabled account or a changed role takes effect immediately.
pub async fn user_from_token(state: &AppState, token: &str) -> AppResult<User> {
    let claims = auth::decode_token(token, &state.config.jwt_secret)?;
    users::find_by_id(&state.db, claims.sub)
        .await?
        .filter(|u| u.is_active)
        .ok_or(AppError::Unauthorized)
}

/// `Set-Cookie` value for a new session.
pub fn session_cookie(token: &str, state: &AppState) -> String {
    let secure = if state.config.cookie_secure { "; Secure" } else { "" };
    format!(
        "{COOKIE_NAME}={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={}{secure}",
        state.config.jwt_ttl_minutes * 60
    )
}

pub fn clear_cookie() -> String {
    format!("{COOKIE_NAME}=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0")
}

/// Extracts the session token from a `Cookie` header value.
pub fn token_from_cookie_header(header: &str) -> Option<&str> {
    header
        .split(';')
        .filter_map(|kv| kv.trim().split_once('='))
        .find(|(k, _)| *k == COOKIE_NAME)
        .map(|(_, v)| v)
        .filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cookie_header() {
        assert_eq!(token_from_cookie_header("a=1; lib_session=abc.def; b=2"), Some("abc.def"));
        assert_eq!(token_from_cookie_header("lib_session="), None);
        assert_eq!(token_from_cookie_header("other=1"), None);
    }
}
