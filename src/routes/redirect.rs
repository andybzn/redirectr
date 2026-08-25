//! `src/routes/redirect.rs`
//! Handlers for creating short links and redirecting to their targets.
//!
//! [`get`] resolves a code, [`post`] creates one, and [`default`] serves the
//! configured fallback for `GET /`.
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderName, StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;
use tracing::{Level, event};

/// Looks up a short-link code and redirects to its stored URL.
///
/// On success, the response has a `302 Found` status and a `Location` header
/// containing the URL associated with `code`.
///
/// # Errors
///
/// Returns [`StatusCode::NOT_FOUND`] when no link exists for `code`, or
/// [`StatusCode::INTERNAL_SERVER_ERROR`] when the database query fails.
///
/// # Arguments
///
/// * `code` - Short-link code to look up.
/// * `pool` - `SQLite` connection pool used to query the `links` table.
pub async fn get(
    Path(code): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<impl IntoResponse, StatusCode> {
    event!(Level::DEBUG, "Received code: {code}");

    let target: Option<String> = sqlx::query_scalar!("SELECT url FROM links WHERE code = ?", code)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    event!(Level::DEBUG, "Returning URL for code {code}: {target:?}");

    target.map_or_else(
        || Err(StatusCode::NOT_FOUND),
        |t| Ok((StatusCode::FOUND, [(header::LOCATION, t)])),
    )
}

/// Query parameters used to create a short link.
#[derive(Deserialize)]
pub struct RedirectParams {
    /// Short-link code to create.
    code: String,
    /// Target URL the code redirects to.
    url: String,
}

/// Creates a short link from query parameters.
///
/// The `url` is validated and normalized by [`crate::helpers::validate_url`]
/// before it is stored. Successful inserts return `201 Created`.
///
/// The request is expected to provide `code` and `url` query parameters, for
/// example: `/?code=docs&url=https%3A%2F%2Fexample.com`.
///
/// # Errors
///
/// Returns [`StatusCode::BAD_REQUEST`] when `url` is invalid or does not use
/// the `http` or `https` scheme. Returns [`StatusCode::CONFLICT`] when `code`
/// already exists, or [`StatusCode::INTERNAL_SERVER_ERROR`] for another
/// database error.
///
/// # Arguments
///
/// * `params` - Query parameters containing the new link code and target URL.
/// * `pool` - `SQLite` connection pool used to insert the link.
pub async fn post(
    params: Query<RedirectParams>,
    State(pool): State<SqlitePool>,
) -> Result<StatusCode, StatusCode> {
    event!(Level::DEBUG, "Received code: {}", params.code);

    let url: String = crate::helpers::validate_url(&params.url)?;

    let res = sqlx::query!(
        "INSERT INTO links (code, url) VALUES ($1, $2)",
        &params.code,
        &url
    )
    .execute(&pool)
    .await;

    crate::helpers::match_insert_result(res)
}

/// Deletes the short link identified by `code`.
///
/// Successful deletions return `204 No Content`. If no link exists for
/// `code`, the operation still succeeds with `204 No Content`.
///
/// # Errors
///
/// Returns [`StatusCode::INTERNAL_SERVER_ERROR`] when the database query
/// fails.
///
/// # Arguments
///
/// * `code` - Short-link code to delete.
/// * `pool` - `SQLite` connection pool used to delete the link.
pub async fn delete(
    Path(code): Path<String>,
    State(pool): State<SqlitePool>,
) -> Result<StatusCode, StatusCode> {
    event!(Level::DEBUG, "DELETE: Received code: {code}");

    sqlx::query!("DELETE FROM links WHERE code == $1", code)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Updates the target URL for an existing short link.
///
/// The `url` is validated and normalized by [`crate::helpers::validate_url`]
/// before it is stored. Successful updates return `204 No Content`.
///
/// # Errors
///
/// Returns [`StatusCode::BAD_REQUEST`] when `url` is invalid or does not use
/// the `http` or `https` scheme. Returns [`StatusCode::NOT_FOUND`] when no
/// link exists for `code`, or [`StatusCode::INTERNAL_SERVER_ERROR`] for a
/// database error.
///
/// # Arguments
///
/// * `params` - Query parameters containing the link code and target URL.
/// * `pool` - `SQLite` connection pool used to update the link.
pub async fn patch(
    params: Query<RedirectParams>,
    State(pool): State<SqlitePool>,
) -> Result<StatusCode, StatusCode> {
    event!(Level::DEBUG, "PATCH: Received code: {}", params.code);

    let validated_url: String = crate::helpers::validate_url(&params.url)?;

    let validate_code_exists = sqlx::query!("SELECT url FROM links WHERE code = $1", &params.code)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if validate_code_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    sqlx::query!(
        "UPDATE links SET url = $1 WHERE code = $2",
        &validated_url,
        &params.code,
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Redirects to the configured default URL.
///
/// This is the handler for `GET /`, giving requests that carry no short-link
/// code somewhere to land. The destination comes from
/// [`crate::Config::default_url`].
///
/// Returns a `302 Found` response with `url` in the `Location` header. Unlike
/// the stored targets, `url` is passed through unvalidated.
///
/// # Arguments
///
/// * `url` - Default destination URL.
#[allow(clippy::unused_async)]
pub async fn default(url: Arc<str>) -> (StatusCode, [(HeaderName, String); 1]) {
    (StatusCode::FOUND, [(header::LOCATION, url.to_string())])
}
