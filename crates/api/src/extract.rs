use axum::{
    Json,
    extract::{FromRequest, FromRequestParts, Request},
    http::{header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
};
use domain::models::Role;
use infra::{AppError, AppState, service::session};
use serde::de::DeserializeOwned;
use uuid::Uuid;
use validator::Validate;

/// Authenticated staff user, taken from `Authorization: Bearer <jwt>`.
#[derive(Debug, Clone, Copy)]
pub struct AuthUser {
    pub id: Uuid,
    pub role: Role,
}

impl AuthUser {
    pub fn require(&self, role: Role) -> Result<(), AppError> {
        if self.role.satisfies(role) { Ok(()) } else { Err(AppError::Forbidden) }
    }
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        // Loads the user from the DB: disabled accounts / role changes apply immediately.
        let user = session::user_from_token(state, token).await?;
        Ok(AuthUser { id: user.id, role: user.role })
    }
}

/// Like [`AuthUser`] but also accepts the web UI's session cookie, so `<img>`
/// tags on staff pages can load protected photos. Only used on GET routes.
#[derive(Debug, Clone, Copy)]
pub struct StaffAuth(pub AuthUser);

impl FromRequestParts<AppState> for StaffAuth {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        let cookie = parts
            .headers
            .get(axum::http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(session::token_from_cookie_header);
        let token = bearer.or(cookie).ok_or(AppError::Unauthorized)?;
        let user = session::user_from_token(state, token).await?;
        Ok(StaffAuth(AuthUser { id: user.id, role: user.role }))
    }
}

/// `Json<T>` that also runs `validator` rules; failures become 422 with field errors.
pub struct ValidJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await.map_err(IntoResponse::into_response)?;
        value.validate().map_err(|e| AppError::from(e).into_response())?;
        Ok(ValidJson(value))
    }
}
