//! `src/lib.rs`
//! HTTP routing for the `redirectr` short-link service.
//!
//! The [`app`] function builds the application router. It provides endpoints
//! for creating and resolving short links, checking database health, and
//! redirecting to a configured default URL.
//!
//! Configuration is read from the environment by [`Config`], and the binary in
//! `src/main.rs` wires that configuration into [`app`].
pub mod config;
pub mod helpers;
pub mod middleware;
pub mod routes;

pub use config::Config;

use axum::{
    Router,
    middleware::from_fn,
    routing::{get, post},
};
use std::sync::Arc;

/// Builds the application [`Router`].
///
/// The returned router exposes the following endpoints:
///
/// - `GET /health` checks whether the `links` table can be queried.
/// - `GET /{code}` looks up a short-link code and redirects to its target, or
///   returns `404 Not Found` when the code does not exist.
/// - `POST /?code=<code>&url=<url>` creates a short link. This endpoint
///   requires an `Authorization` header in the form `Bearer <admin_token>`.
/// - `GET /` redirects to `default_url`.
///
/// The provided [`sqlx::Pool`] is made available to the handlers through Axum's
/// router state. The `admin_token` is captured by the `POST /` middleware and
/// the `default_url` is captured by the `GET /` handler.
///
/// # Arguments
///
/// * `connection_pool` - `SQLite` connection pool used by the health, creation,
///   and lookup handlers.
/// * `admin_token` - Token required by the `Authorization: Bearer ...` header
///   for link creation.
/// * `default_url` - URL returned as the `Location` header for `GET /`.
pub fn app(
    connection_pool: sqlx::Pool<sqlx::Sqlite>,
    admin_token: Arc<str>,
    default_url: Arc<str>,
) -> Router {
    Router::new()
        .route("/health", get(routes::health::get))
        .route("/{code}", get(routes::get))
        .route(
            "/",
            post(routes::post).layer(from_fn(move |req, next| {
                let token: Arc<str> = Arc::clone(&admin_token);
                async move { middleware::check_token(req, next, token).await }
            })),
        )
        .route(
            "/",
            get(move || {
                let url: Arc<str> = Arc::clone(&default_url);
                async move { routes::default(url).await }
            }),
        )
        .with_state(connection_pool)
}
