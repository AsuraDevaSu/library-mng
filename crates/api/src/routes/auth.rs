use axum::{Json, extract::State};
use domain::{
    dto::{LoginRequest, LoginResponse},
    models::User,
};
use infra::{AppError, AppResult, AppState, repo::users, service::session};

use crate::{AuthUser, ValidJson};

pub async fn login(
    State(state): State<AppState>,
    ValidJson(req): ValidJson<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let (user, token) = session::login(&state, &req.email, &req.password).await?;
    let ttl = state.config.jwt_ttl_minutes;
    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer".into(),
        expires_in: ttl * 60,
        user,
    }))
}

pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<User>> {
    users::find_by_id(&state.db, user.id)
        .await?
        .filter(|u| u.is_active)
        .map(Json)
        .ok_or(AppError::Unauthorized)
}
