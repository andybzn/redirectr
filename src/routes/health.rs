//! `src/routes/health.rs`
//! Handler for the `GET /health` endpoint.
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use sqlx::sqlite::SqlitePool;

/// Checks whether the `links` table can be queried.
///
/// This probes the schema, not just the connection, so a reachable database
/// with missing migrations is still reported as unhealthy.
///
/// Returns `200 OK` if the query succeeds, or `503 Service Unavailable`
/// otherwise.
///
/// # Arguments
///
/// * `pool` - `SQLite` connection pool used to probe the `links` table.
pub async fn get(State(pool): State<SqlitePool>) -> impl IntoResponse {
    match sqlx::query("SELECT 1 FROM links LIMIT 1")
        .fetch_optional(&pool)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}
