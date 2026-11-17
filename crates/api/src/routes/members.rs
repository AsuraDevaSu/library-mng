use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use domain::{
    dto::{CreateMemberRequest, SearchQuery},
    models::{Member, Role},
};
use infra::{AppResult, AppState, repo::members};

use crate::{AuthUser, ValidJson};

pub async fn search(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<Vec<Member>>> {
    user.require(Role::Librarian)?;
    Ok(Json(members::search(&state.db, &q.q, q.limit, q.offset).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    ValidJson(req): ValidJson<CreateMemberRequest>,
) -> AppResult<(StatusCode, Json<Member>)> {
    user.require(Role::Librarian)?;
    Ok((StatusCode::CREATED, Json(members::create(&state.db, &req, user.id).await?)))
}
