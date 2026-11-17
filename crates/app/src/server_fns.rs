//! Public server functions (no login needed). They reuse the same `infra`
//! code as the REST API. Staff-only functions live in `admin_fns` / `auth`.

use domain::models::{BookSummary, SiteInfo};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use crate::auth::ssr::to_sfe;

#[server]
pub async fn search_books(q: String) -> Result<Vec<BookSummary>, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    infra::repo::books::search(&state.db, &q, 60, 0).await.map_err(to_sfe)
}

#[server]
pub async fn get_site_info() -> Result<SiteInfo, ServerFnError> {
    let state = expect_context::<infra::AppState>();
    infra::repo::books::site_info(&state.db).await.map_err(to_sfe)
}
