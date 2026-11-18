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

/// CSV of every bag still at the desk (tag, name, ID card, colour, time). No photos.
pub async fn export_csv(State(state): State<AppState>, StaffAuth(user): StaffAuth) -> AppResult<impl IntoResponse> {
    user.require(Role::DeskStaff)?;
    let bags = bags::export_active(&state.db).await?;
    infra::repo::audit::log(&state.db, user.id, "bag.export", "bag_deposit", Uuid::nil(), serde_json::json!({ "rows": bags.len() }))
        .await?;
    let file = format!("bags-at-desk-{}.csv", chrono::Local::now().format("%Y-%m-%d-%H%M"));
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{file}\"")),
            (header::CACHE_CONTROL, "private, no-store".to_string()),
        ],
        bags::to_csv(&bags),
    ))
}
