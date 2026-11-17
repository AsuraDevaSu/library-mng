use axum::{Json, extract::State, http::StatusCode};
use infra::AppState;
use serde_json::{Value, json};

pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let db_ok = sqlx_ping(&state).await;
    let status = if db_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };
    (status, Json(json!({ "status": if db_ok { "ok" } else { "degraded" }, "database": db_ok })))
}

async fn sqlx_ping(state: &AppState) -> bool {
    infra::repo::users::count(&state.db).await.is_ok()
}
