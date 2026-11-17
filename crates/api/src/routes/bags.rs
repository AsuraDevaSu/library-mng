use axum::{
    extract::{Path, State},
    http::header,
    response::IntoResponse,
};
use domain::models::Role;
use infra::{AppError, AppResult, AppState, service::bags};
use uuid::Uuid;

use crate::StaffAuth;

/// Face or bag photo of a deposit. Desk staff and above only; never cached.
pub async fn photo(
    State(state): State<AppState>,
    StaffAuth(user): StaffAuth,
    Path((id, kind)): Path<(Uuid, String)>,
) -> AppResult<impl IntoResponse> {
    user.require(Role::DeskStaff)?;
    let path = bags::photo_path(&state, id, kind.parse()?).await?;
    let bytes = tokio::fs::read(&path).await.map_err(|_| AppError::NotFound("photo"))?;
    Ok((
        [(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "private, no-store")],
        bytes,
    ))
}
