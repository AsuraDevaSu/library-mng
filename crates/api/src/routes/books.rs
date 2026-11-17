use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use domain::{
    dto::{CreateBookRequest, SearchQuery},
    models::{Book, BookSummary, Copy, Role},
};
use infra::{AppResult, AppState, repo::books};
use serde::Serialize;
use uuid::Uuid;

use crate::{AuthUser, ValidJson};

/// Public catalog search.
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<Vec<BookSummary>>> {
    Ok(Json(books::search(&state.db, &q.q, q.limit, q.offset).await?))
}

#[derive(Serialize)]
pub struct BookDetail {
    #[serde(flatten)]
    book: Book,
    copies: Vec<Copy>,
}

pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Json<BookDetail>> {
    let book = books::get(&state.db, id).await?;
    let copies = books::copies(&state.db, id).await?;
    Ok(Json(BookDetail { book, copies }))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    ValidJson(req): ValidJson<CreateBookRequest>,
) -> AppResult<(StatusCode, Json<Book>)> {
    user.require(Role::Librarian)?;
    Ok((StatusCode::CREATED, Json(books::create(&state.db, &req, user.id).await?)))
}
