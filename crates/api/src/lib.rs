//! REST API mounted at `/api/v1`. Stable, JSON, bearer-token auth —
//! this is the contract the Flutter app (and any other client) uses.

mod extract;
mod routes;

use axum::{
    Router,
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
        .route("/books/{id}", get(books::get))
        .route("/members", get(members::search).post(members::create))
        .route("/circulation/checkout", post(circulation::checkout))
        .route("/circulation/return", post(circulation::return_copy))
        .route("/loans/{id}/renew", post(circulation::renew))
        .route("/bags/{id}/photo/{kind}", get(bags::photo))
        .with_state(state)
}
