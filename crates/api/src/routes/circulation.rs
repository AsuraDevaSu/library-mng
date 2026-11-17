use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use domain::{
    dto::{CheckoutRequest, ReturnRequest},
    models::{Loan, ReturnReceipt, Role},
};
use infra::{AppResult, AppState, service::circulation};
use uuid::Uuid;

use crate::{AuthUser, ValidJson};

pub async fn checkout(
    State(state): State<AppState>,
    user: AuthUser,
    ValidJson(req): ValidJson<CheckoutRequest>,
) -> AppResult<(StatusCode, Json<Loan>)> {
    user.require(Role::Librarian)?;
    Ok((StatusCode::CREATED, Json(circulation::checkout(&state.db, &req, user.id).await?)))
}

pub async fn return_copy(
    State(state): State<AppState>,
    user: AuthUser,
    ValidJson(req): ValidJson<ReturnRequest>,
) -> AppResult<Json<ReturnReceipt>> {
    user.require(Role::Librarian)?;
    Ok(Json(circulation::return_copy(&state.db, &req, user.id).await?))
}

pub async fn renew(
    State(state): State<AppState>,
    user: AuthUser,
    Path(loan_id): Path<Uuid>,
) -> AppResult<Json<Loan>> {
    user.require(Role::Librarian)?;
    Ok(Json(circulation::renew(&state.db, loan_id, user.id).await?))
}
