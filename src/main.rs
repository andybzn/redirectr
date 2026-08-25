//! `src/main.rs`
//! Binary entry point for the `redirectr` short-link service.
//!
//! Reads configuration from the environment, opens the `SQLite` pool and runs
//! the migrations in `./migrations`, then serves the router built by
//! [`redirectr::app`].
use redirectr::config::Config;
use sqlx::{
    Pool,
    sqlite::{Sqlite, SqliteConnectOptions, SqlitePoolOptions},
};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{Level, event};
use tracing_subscriber::filter::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter: EnvFilter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("redirectr=debug,sqlx=warn"));

    tracing_subscriber::fmt().with_env_filter(filter).init();

    event!(Level::INFO, "Redirectr starting...");

    let config: Config = Config::new()?;
    let admin_token: Arc<str> = Arc::from(config.token.as_str());
    let default_url: Arc<str> = Arc::from(config.default_url.as_str());

    let options: SqliteConnectOptions =
        SqliteConnectOptions::from_str(&config.database_url)?.create_if_missing(true);

    let pool: Pool<Sqlite> = SqlitePoolOptions::new()
        .max_connections(config.max_connections)
        .connect_with(options)
        .await?;

    event!(Level::INFO, "Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    event!(Level::INFO, "Database migrations completed successfully.");

    event!(Level::INFO, "Spawning redirectr server...");
    let app: axum::Router = redirectr::app(pool, &admin_token, default_url);
    let address: String = format!("{0}:{1}", config.server_ip, config.server_port);
    let listener: TcpListener = TcpListener::bind(&address).await?;
    event!(Level::INFO, "Server listening on {address}");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
