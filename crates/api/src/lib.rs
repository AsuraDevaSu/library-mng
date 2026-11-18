//! REST API mounted at `/api/v1`. Stable, JSON, bearer-token auth —
//! this is the contract the Flutter app (and any other client) uses.

mod extract;
mod routes;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use infra::AppState;

pub use extract::{AuthUser, StaffAuth, ValidJson};

pub fn router(state: AppState) -> Router {
    use routes::*;
    Router::new()
        .route("/health", get(health::health))
        .route("/auth/login", post(auth::login))
        .route("/me", get(auth::me))
        .route("/books", get(books::search).post(books::create))
        .route("/books/categories", get(books::categories))
        .route("/books/{id}", get(books::get).put(books::update).delete(books::delete))
        .route("/books/{id}/copies", post(books::add_copies))
        .route("/books/{id}/cover", get(books::cover))
        .route("/books/{id}/ebook", get(books::ebook))
        .route("/members", get(members::search).post(members::create))
        .route("/circulation/checkout", post(circulation::checkout))
        .route("/circulation/return", post(circulation::return_copy))
        .route("/loans/{id}/renew", post(circulation::renew))
        .route("/bags/export.csv", get(bags::export_csv))
        .route("/bags/{id}/photo/{kind}", get(bags::photo))
        .with_state(state)
}

/// Large uploads (book PDFs). Mounted at `/api/v1` next to [`router`], but
/// outside the global request timeout, with its own body limit and timeout.
pub fn upload_router(state: AppState) -> Router {
    use routes::books;
    Router::new()
        .route("/books/{id}/files/{kind}", post(books::upload_file).delete(books::remove_file))
        .layer(DefaultBodyLimit::max(infra::service::book_files::EBOOK_MAX_BYTES as usize + 64 * 1024))
        .with_state(state)
}
