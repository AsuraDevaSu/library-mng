//! Public server functions (no login needed). They reuse the same `infra`
//! code as the REST API. Staff-only functions live in `admin_fns` / `auth`.

use domain::models::{BookDetail, BookSummary, SiteInfo};
use leptos::prelude::*;
use uuid::Uuid;

#[cfg(feature = "ssr")]
use crate::auth::ssr::to_sfe;

#[server]
pub async fn search_books(q: String, category: String, online: bool) -> Result<Vec<BookSummary>, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    infra::repo::books::search(&state.db, &q, &category, online, 60, 0).await.map_err(to_sfe)
}

#[server]
pub async fn book_categories() -> Result<Vec<String>, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    infra::repo::books::categories(&state.db).await.map_err(to_sfe)
}

#[server]
pub async fn get_book(id: Uuid) -> Result<BookDetail, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    let book = infra::repo::books::get(&state.db, id).await.map_err(to_sfe)?;
    let copies = infra::repo::books::copies(&state.db, id).await.map_err(to_sfe)?;
    Ok(BookDetail { book, copies })
}

#[server]
pub async fn get_site_info() -> Result<SiteInfo, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    infra::repo::books::site_info(&state.db).await.map_err(to_sfe)
}
