use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use domain::{
    dto::{AddCopiesRequest, CreateBookRequest, SearchQuery, UpdateBookRequest},
    models::{Book, BookDetail, BookSummary, Copy, Role},
};
use infra::{
    AppError, AppResult, AppState,
    repo::books,
    service::book_files::{self, BookFile, Upload},
};
use tower::ServiceExt;
use tower_http::services::ServeFile;
use uuid::Uuid;

use crate::{AuthUser, StaffAuth, ValidJson};

/// Public catalog search (`?q=&category=&online=true`).
pub async fn search(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> AppResult<Json<Vec<BookSummary>>> {
    Ok(Json(books::search(&state.db, &q.q, &q.category, q.online, q.limit, q.offset).await?))
}

/// Public list of categories that have books.
pub async fn categories(State(state): State<AppState>) -> AppResult<Json<Vec<String>>> {
    Ok(Json(books::categories(&state.db).await?))
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

pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    ValidJson(req): ValidJson<UpdateBookRequest>,
) -> AppResult<Json<Book>> {
    user.require(Role::Librarian)?;
    Ok(Json(books::update(&state.db, id, &req, user.id).await?))
}

pub async fn delete(State(state): State<AppState>, user: AuthUser, Path(id): Path<Uuid>) -> AppResult<StatusCode> {
    user.require(Role::Librarian)?;
    book_files::delete_book(&state, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_copies(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    ValidJson(req): ValidJson<AddCopiesRequest>,
) -> AppResult<Json<Vec<Copy>>> {
    user.require(Role::Librarian)?;
    Ok(Json(books::add_copies(&state.db, id, &req, user.id).await?))
}

/// Public cover image.
pub async fn cover(State(state): State<AppState>, Path(id): Path<Uuid>) -> AppResult<Response> {
    let (path, mime) = book_files::stored(&state, id, BookFile::Cover).await?;
    let bytes = tokio::fs::read(&path).await.map_err(|_| AppError::NotFound("file"))?;
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            // URLs carry ?v=<updated_at>, so a new upload gets a new URL.
            (header::CACHE_CONTROL, "public, max-age=86400".into()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".into()),
        ],
        bytes,
    )
        .into_response())
}

/// Public e-book, shown inline by the browser's PDF viewer. Supports range
/// requests, so large PDFs start showing before they finish downloading.
pub async fn ebook(State(state): State<AppState>, Path(id): Path<Uuid>, req: Request) -> AppResult<Response> {
    let (path, _) = book_files::stored(&state, id, BookFile::Ebook).await?;
    let mut res = ServeFile::new(path)
        .oneshot(req)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .map(Body::new);
    if res.status() == StatusCode::NOT_FOUND {
        return Err(AppError::NotFound("file"));
    }
    let h = res.headers_mut();
    h.insert(header::CONTENT_DISPOSITION, "inline".parse().unwrap());
    h.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    h.insert(header::CACHE_CONTROL, "public, max-age=300".parse().unwrap());
    Ok(res)
}

/// Rejects cookie-authenticated writes coming from another site.
fn same_origin(headers: &HeaderMap) -> AppResult<()> {
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok());
    match (origin, host) {
        (Some(o), Some(h)) if o.split("://").nth(1) != Some(h) => Err(AppError::Forbidden),
        _ => Ok(()),
    }
}

/// Upload a cover (`kind = cover`) or PDF (`kind = ebook`) as multipart field `file`.
/// Accepts the staff session cookie, so the admin page can post directly.
pub async fn upload_file(
    State(state): State<AppState>,
    StaffAuth(user): StaffAuth,
    Path((id, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
    mut form: Multipart,
) -> AppResult<StatusCode> {
    user.require(Role::Librarian)?;
    same_origin(&headers)?;
    let kind: BookFile = kind.parse()?;
    let bad = |e: axum::extract::multipart::MultipartError| AppError::Conflict(format!("Upload failed: {}", e.body_text()));
    while let Some(mut field) = form.next_field().await.map_err(bad)? {
        if field.name() != Some("file") {
            continue;
        }
        let mut upload = Upload::begin(&state, id, kind).await?;
        while let Some(chunk) = field.chunk().await.map_err(bad)? {
            upload.write(&chunk).await?;
        }
        upload.finish(&state, user.id).await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    Err(AppError::Conflict("Choose a file to upload.".into()))
}

pub async fn remove_file(
    State(state): State<AppState>,
    StaffAuth(user): StaffAuth,
    Path((id, kind)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    user.require(Role::Librarian)?;
    same_origin(&headers)?;
    book_files::remove(&state, id, kind.parse()?, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
