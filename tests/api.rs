//! `tests/api.rs`
//! End-to-end tests driving the router built by `redirectr::app`.
//!
//! Each test builds a fresh pool, runs the migrations, and sends requests
//! through the real router with `tower`'s `oneshot`. Pools are in-memory except
//! in `links_survive_reconnect`, which needs a file to reopen.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::sync::Arc;
use tower::ServiceExt; // .oneshot()

async fn test_pool() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    pool
}

fn router(pool: SqlitePool) -> axum::Router {
    redirectr::app(
        pool,
        Arc::from("realtoken"),
        Arc::from("https://default.example"),
    )
}

fn enc(s: &str) -> String {
    s.replace(':', "%3A")
        .replace('/', "%2F")
        .replace('?', "%3F")
        .replace('&', "%26")
}

async fn post(pool: &SqlitePool, auth: Option<&str>, code: &str, url: &str) -> StatusCode {
    let mut req = Request::builder()
        .method("POST")
        .uri(format!("/?code={code}&url={}", enc(url)));
    if let Some(a) = auth {
        req = req.header("Authorization", a);
    }
    router(pool.clone())
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

async fn get(pool: &SqlitePool, uri: &str) -> axum::response::Response {
    router(pool.clone())
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn correct_token_creates_link() {
    let pool = test_pool().await;
    let s = post(&pool, Some("Bearer realtoken"), "a1", "https://ok.example").await;
    assert_eq!(s, StatusCode::CREATED, "correct token must create");
}

#[tokio::test]
async fn bad_tokens_rejected() {
    let pool = test_pool().await;
    for auth in [
        Some("Bearer nope"),
        Some("Bearer "),
        Some("realtoken"),
        None,
    ] {
        assert_eq!(
            post(&pool, auth, "a2", "https://ok.example").await,
            StatusCode::UNAUTHORIZED,
            "must reject {auth:?}"
        );
    }
}

#[tokio::test]
async fn non_http_schemes_rejected() {
    let pool = test_pool().await;
    for bad in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "ftp://x.example/f",
        "data:text/html,x",
        "notaurl",
    ] {
        assert_eq!(
            post(&pool, Some("Bearer realtoken"), "s1", bad).await,
            StatusCode::BAD_REQUEST,
            "must reject {bad}"
        );
    }
}

#[tokio::test]
async fn http_and_https_accepted() {
    let pool = test_pool().await;
    assert_eq!(
        post(&pool, Some("Bearer realtoken"), "h1", "http://ok.example").await,
        StatusCode::CREATED
    );
    assert_eq!(
        post(&pool, Some("Bearer realtoken"), "h2", "https://ok.example").await,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn links_survive_reconnect() {
    let dir = std::env::temp_dir().join(format!("redirectr-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let url = format!("sqlite://{}/t.sqlite?mode=rwc", dir.display());

    let pool = SqlitePoolOptions::new().connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    assert_eq!(
        post(
            &pool,
            Some("Bearer realtoken"),
            "keep",
            "https://ok.example"
        )
        .await,
        StatusCode::CREATED
    );
    pool.close().await;

    // fresh pool, same file
    let pool2 = SqlitePoolOptions::new().connect(&url).await.unwrap();
    let res = router(pool2)
        .oneshot(Request::builder().uri("/keep").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(
        res.status(),
        StatusCode::FOUND,
        "link must survive reconnect"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn redirect_returns_302_with_location() {
    let pool = test_pool().await;
    post(&pool, Some("Bearer realtoken"), "go", "https://ok.example").await;
    let res = get(&pool, "/go").await;
    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(res.headers()["location"], "https://ok.example/");
}

#[tokio::test]
async fn unknown_code_is_404() {
    let pool = test_pool().await;
    let res = get(&pool, "/nope").await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn health_ok_on_live_db() {
    let pool = test_pool().await;
    let res = get(&pool, "/health").await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn root_redirects_to_default_url() {
    let pool = test_pool().await;
    let res = get(&pool, "/").await;
    assert_eq!(res.status(), StatusCode::FOUND);
    assert_eq!(res.headers()["location"], "https://default.example");
}

#[tokio::test]
async fn duplicate_code_does_not_repoint() {
    let pool = test_pool().await;
    post(&pool, Some("Bearer realtoken"), "dup", "https://a.example").await; // 201
    let status = post(&pool, Some("Bearer realtoken"), "dup", "https://b.example").await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "second write must be rejected"
    );

    let res = get(&pool, "/dup").await;
    assert_eq!(
        res.headers()["location"],
        "https://a.example/",
        "original target must survive the rejected overwrite"
    );
}

/// `created_at` is populated by the migration's `DEFAULT (datetime('now','utc'))`.
/// Asserts `SQLite` can actually parse what got stored, and that the value is real
/// UTC rather than shifted by the host timezone.
#[tokio::test]
async fn created_at_is_parseable_utc() {
    let pool = test_pool().await;
    post(&pool, Some("Bearer realtoken"), "ts", "https://ok.example").await;

    let (parsed, skew): (Option<String>, i64) = sqlx::query_as(
        "SELECT datetime(created_at), abs(strftime('%s', created_at) - strftime('%s', 'now'))
         FROM links WHERE code = 'ts'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        parsed.is_some(),
        "SQLite datetime() must parse created_at, got NULL"
    );
    assert!(
        skew < 60,
        "created_at must be current UTC, off by {skew}s (timezone shift?)"
    );
}

#[tokio::test]
async fn db_error_is_500_not_404() {
    let pool = test_pool().await;
    sqlx::query("DROP TABLE links")
        .execute(&pool)
        .await
        .unwrap();

    let res = get(&pool, "/anything").await;
    assert_eq!(
        res.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "a broken DB must not masquerade as 404"
    );
}

#[tokio::test]
async fn health_fails_when_schema_is_missing() {
    let pool = test_pool().await;
    sqlx::query("DROP TABLE links")
        .execute(&pool)
        .await
        .unwrap();

    let res = get(&pool, "/health").await;
    assert_eq!(
        res.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "health must probe the links table, not just the connection"
    );
}
