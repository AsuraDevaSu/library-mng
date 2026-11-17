//! Staff sessions for the web UI: an HttpOnly cookie holding the same JWT the
//! REST API uses. Every server function that touches staff data calls
//! `ssr::require(role)` — hiding a page in the UI is not access control.

use domain::models::User;
use leptos::prelude::*;

/// Shared auth state, provided once in `App`.
#[derive(Clone, Copy)]
pub struct AuthCtx {
    pub user: Resource<Result<Option<User>, ServerFnError>>,
    pub login: ServerAction<Login>,
    pub logout: ServerAction<Logout>,
}

impl AuthCtx {
    pub fn new() -> Self {
        let login = ServerAction::<Login>::new();
        let logout = ServerAction::<Logout>::new();
        let user = Resource::new(
            move || (login.version().get(), logout.version().get()),
            |_| current_user(),
        );
        Self { user, login, logout }
    }
}

impl Default for AuthCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// Human-readable message from a server function error.
pub fn err_msg(e: &ServerFnError) -> String {
    match e {
        ServerFnError::ServerError(m) => m.clone(),
        other => other.to_string(),
    }
}

#[server]
pub async fn current_user() -> Result<Option<User>, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    ssr::current(&state).await
}

#[server]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    use axum::http::{HeaderValue, header::SET_COOKIE};
    use domain::models::Role;
    use infra::{AppError, service::session};

    let state = expect_context::<infra::AppState>();
    let (user, token) = session::login(&state, &email, &password).await.map_err(|e| match e {
        AppError::Unauthorized => ServerFnError::new("Wrong email or password, or the account is disabled."),
        e => ssr::to_sfe(e),
    })?;
    let cookie = HeaderValue::from_str(&session::session_cookie(&token, &state))
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    expect_context::<leptos_axum::ResponseOptions>().append_header(SET_COOKIE, cookie);
    leptos_axum::redirect(if user.role == Role::Admin { "/admin" } else { "/" });
    Ok(())
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use axum::http::{HeaderValue, header::SET_COOKIE};
    let cookie = HeaderValue::from_str(&infra::service::session::clear_cookie())
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    expect_context::<leptos_axum::ResponseOptions>().append_header(SET_COOKIE, cookie);
    leptos_axum::redirect("/");
    Ok(())
}

#[cfg(feature = "ssr")]
pub mod ssr {
    use axum::http::{HeaderMap, header::COOKIE};
    use domain::models::{Role, User};
    use infra::{AppError, AppState, service::session};
    use leptos::prelude::*;

    /// Maps backend errors to messages safe to show in the UI.
    pub fn to_sfe(e: AppError) -> ServerFnError {
        match e {
            AppError::Validation(fields) => {
                let names = fields
                    .as_object()
                    .map(|o| o.keys().map(|k| k.replace('_', " ")).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                ServerFnError::new(format!("Please check: {names}"))
            }
            AppError::Db(_) | AppError::Internal(_) => {
                tracing::error!(error = ?e, "server function failed");
                ServerFnError::new("Something went wrong. Please try again.")
            }
            other => ServerFnError::new(other.to_string()),
        }
    }

    pub async fn current(state: &AppState) -> Result<Option<User>, ServerFnError> {
        let headers: HeaderMap = leptos_axum::extract().await.map_err(|e| ServerFnError::new(e.to_string()))?;
        let Some(token) = headers
            .get(COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(session::token_from_cookie_header)
        else {
            return Ok(None);
        };
        match session::user_from_token(state, token).await {
            Ok(user) => Ok(Some(user)),
            Err(AppError::Unauthorized) => Ok(None),
            Err(e) => Err(to_sfe(e)),
        }
    }

    /// The logged-in user, if they have at least `role`.
    pub async fn require(role: Role) -> Result<(AppState, User), ServerFnError> {
        let state = expect_context::<AppState>();
        let user = current(&state).await?.ok_or_else(|| ServerFnError::new("Please log in."))?;
        if !user.role.satisfies(role) {
            return Err(ServerFnError::new("You don't have permission to do this."));
        }
        Ok((state, user))
    }

    /// Runs `validator` rules on server function input.
    pub fn validate(input: &impl validator::Validate) -> Result<(), ServerFnError> {
        input.validate().map_err(|e| to_sfe(AppError::from(e)))
    }
}
