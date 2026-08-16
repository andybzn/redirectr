//! `src/helpers.rs`
//! Shared helpers for the route handlers.
//!
//! [`validate_url`] checks a redirect target before it is stored, and
//! [`match_insert_result`] turns a `sqlx` insert result into an HTTP status.
use axum::http::StatusCode;
use tracing::{Level, event};
use url::Url;

/// Validates a URL for use as a redirect target.
///
/// Only `http` and `https` URLs are accepted. The returned string is the
/// normalized representation produced by `Url::to_string()`, which may differ
/// from the input, for example by appending a trailing slash to a bare host.
///
/// # Errors
///
/// Returns [`StatusCode::BAD_REQUEST`] if `url` cannot be parsed or uses a
/// scheme other than `http` or `https`.
///
/// # Examples
///
/// ```
/// use axum::http::StatusCode;
/// use redirectr::helpers::validate_url;
///
/// assert_eq!(
///     validate_url("https://example.com"),
///     Ok(String::from("https://example.com/")),
/// );
/// assert_eq!(
///     validate_url("javascript:alert(1)"),
///     Err(StatusCode::BAD_REQUEST),
/// );
/// ```
pub fn validate_url(url: &str) -> Result<String, StatusCode> {
    let Ok(parsed) = Url::parse(url) else {
        event!(Level::DEBUG, "Invalid URL: {}", url);
        return Err(StatusCode::BAD_REQUEST);
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        event!(Level::DEBUG, "Invalid URL scheme: {}", parsed.scheme());
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(parsed.to_string())
}

/// Converts the result of inserting a link into its corresponding HTTP status.
///
/// A successful insert is mapped to [`StatusCode::CREATED`]. Database errors
/// caused by a unique constraint violation are mapped to
/// [`StatusCode::CONFLICT`]. All other database or `sqlx` errors are mapped to
/// [`StatusCode::INTERNAL_SERVER_ERROR`].
///
/// # Errors
///
/// Returns [`StatusCode::CONFLICT`] for a unique constraint violation, or
/// [`StatusCode::INTERNAL_SERVER_ERROR`] for any other error.
pub fn match_insert_result(
    result: Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>,
) -> Result<StatusCode, StatusCode> {
    match result {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(sqlx::Error::Database(db_error)) => {
            if db_error.is_unique_violation() {
                event!(
                    Level::DEBUG,
                    "Unique constraint violation: {}",
                    db_error.message()
                );
                Err(StatusCode::CONFLICT)
            } else {
                event!(Level::ERROR, "Database error: {}", db_error.message());
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
        Err(e) => {
            event!(Level::ERROR, "Unexpected error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::validate_url;
    use axum::http::StatusCode;

    #[test]
    fn test_valid_url() {
        assert_eq!(
            Ok(String::from("https://test.com/")),
            validate_url("https://test.com")
        );
    }
    #[test]
    fn test_invalid_url() {
        assert_eq!(
            Err(StatusCode::BAD_REQUEST),
            validate_url("javascript: alert(1)")
        );
    }
}
