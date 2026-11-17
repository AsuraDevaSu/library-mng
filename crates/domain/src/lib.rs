//! Library domain: entities, request DTOs and business rules.
//!
//! This crate must stay pure (no IO, no tokio, no sqlx unless the `sqlx`
//! feature is on) because it is compiled to WASM for the Leptos frontend.

pub mod dto;
pub mod error;
pub mod models;
pub mod rules;

pub use error::DomainError;
