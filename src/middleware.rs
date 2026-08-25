//! `src/middleware.rs`
//! Authentication middleware for protecting routes with a bearer token.
//!
//! [`check_token`] compares the complete `Authorization` header against the
//! expected bearer token by delegating to [`validate_auth_header`]. The
//! comparison uses [`subtle::ConstantTimeEq`]. Unauthorized requests are
//! rejected with `401 Unauthorized`, and logged. If the peer address is
//! available via `ConnectInfo<SocketAddr>`, it is also logged.
use axum::{
    extract::{ConnectInfo, Request},
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, sync::Arc};
use subtle::ConstantTimeEq;
use tracing::{Level, event};

/// Validates the `Authorization` header and runs the rest of the stack when the
/// bearer token matches.
///
/// The header provided must explicitly match `Bearer <token>`. Missing,
/// invalid, or malformed header values, and all non-matching values are
/// rejected.
///
/// Header validation is delegated to [`validate_auth_header`].
///
/// # Errors
///
/// Returns [`StatusCode::UNAUTHORIZED`] when the `Authorization` header is
/// missing, malformed, or does not contain the expected token.
///
/// # Security
///
/// [`validate_auth_header`] uses constant-time equality to mitigate timing
/// attacks.
///
/// # Arguments
///
/// * `req` - Incoming request whose `Authorization` header is checked.
/// * `next` - Remainder of the middleware stack, run only on a match.
/// * `token` - Expected bearer token.
pub async fn check_token(
    req: Request,
    next: Next,
    token: Arc<str>,
) -> Result<Response, StatusCode> {
    let expected_header: String = format!("Bearer {token}");
    match req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|header_value| header_value.to_str().ok())
    {
        Some(header_value) if validate_auth_header(header_value, &expected_header) => {
            Ok(next.run(req).await)
        }
        _ => req
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0)
            .map_or_else(
                || {
                    event!(Level::WARN, "Unauthorized request. Unable to parse Source");
                    Err(StatusCode::UNAUTHORIZED)
                },
                |addr| {
                    event!(Level::WARN, "Unauthorized request. Source: {}", addr);
                    Err(StatusCode::UNAUTHORIZED)
                },
            ),
    }
}

/// Returns whether an incoming `Authorization` header exactly matches the
/// expected bearer token value.
///
/// The comparison operation is performed with [`subtle::ConstantTimeEq`] so
/// that the result does not depend on the position of the first differing
/// byte.
#[must_use]
pub fn validate_auth_header(provided_header: &str, expected_header: &str) -> bool {
    provided_header
        .as_bytes()
        .ct_eq(expected_header.as_bytes())
        .into()
}

#[cfg(test)]
mod tests {
    use crate::middleware::validate_auth_header;

    #[test]
    fn validate_correct_header() {
        assert!(validate_auth_header("expected", "expected"));
    }
    #[test]
    fn reject_incorrect_header() {
        assert!(!validate_auth_header("wrong", "expected"));
    }
}
