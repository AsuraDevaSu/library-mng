use std::{sync::Arc, time::Duration};

use app::{App, shell};
use axum::{
    Router,
    http::{HeaderValue, StatusCode},
};
use infra::{AppState, Config, db};
use leptos::prelude::*;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,server=debug,tower_http=info".into()),
        )
        .init();

    // ---- backend state ----
    let config = Config::from_env()?;
    let pool = db::connect(&config).await?;
    db::migrate(&pool).await?;
    db::bootstrap_admin(&pool, &config).await?;
    let faces = match vision::FaceEngine::load(&config.models_dir) {
        Ok(engine) => {
            tracing::info!(dir = %config.models_dir.display(), "face models loaded");
            Some(Arc::new(engine))
        }
        Err(e) => {
            tracing::warn!(error = %e, "face models not loaded; bag desk will use tag/name pickup only (run scripts/fetch-models.sh)");
            None
        }
    };
    let state = AppState { db: pool, config: Arc::new(config), faces };

    // Hourly: delete bag-desk photos past their retention period.
    tokio::spawn({
        let state = state.clone();
        async move {
            let mut tick = tokio::time::interval(Duration::from_secs(3600));
            loop {
                tick.tick().await;
                match infra::service::bags::cleanup(&state).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(deposits = n, "deleted expired bag photos"),
                    Err(e) => tracing::error!(error = ?e, "bag photo cleanup failed"),
                }
            }
        }
    });

    // ---- leptos ----
    // `cargo leptos` passes the [workspace.metadata.leptos] settings via LEPTOS_* env vars.
    let conf = get_configuration(None)?;
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let web = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            {
                let state = state.clone();
                move || provide_context(state.clone())
            },
            {
                let opts = leptos_options.clone();
                move || shell(opts.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let app = Router::new()
        .nest("/api/v1", api::router(state.clone()).layer(cors(&state.config.cors_origins)))
        .merge(web)
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}  (API: /api/v1)");
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}

/// CORS for API clients on other origins (e.g. Flutter web in dev).
/// Mobile apps don't need CORS; bearer tokens mean no cookies are involved.
fn cors(origins: &[String]) -> CorsLayer {
    let layer = CorsLayer::new().allow_methods(Any).allow_headers(Any);
    if origins.iter().any(|o| o == "*") {
        layer.allow_origin(Any)
    } else {
        layer.allow_origin(
            origins.iter().filter_map(|o| o.parse::<HeaderValue>().ok()).collect::<Vec<_>>(),
        )
    }
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.ok() };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<Option<()>>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutting down");
}
