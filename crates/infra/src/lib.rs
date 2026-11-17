//! Server-side infrastructure shared by the REST API and Leptos server functions.

pub mod auth;
pub mod config;
pub mod db;
pub mod error;
pub mod repo;
pub mod service;
pub mod state;

pub use config::Config;
pub use error::{AppError, AppResult};
pub use state::AppState;
