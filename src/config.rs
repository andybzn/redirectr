//! `src/config.rs`
//! Runtime configuration read from environment variables.
//!
//! [`Config::new`] collects every setting the service needs. `ADMIN_TOKEN` is
//! required; every other setting falls back to a documented default.
use std::env::{self};
use tracing::{Level, event};

/// Runtime settings for the service.
pub struct Config {
    /// TCP port the server listens on. From `SERVER_PORT`, defaults to `3333`.
    pub server_port: String,
    /// IP address the server binds to. From `SERVER_IP`, defaults to `127.0.0.1`.
    pub server_ip: String,
    /// `SQLite` connection URL. From `DATABASE_URL`, defaults to
    /// `sqlite://redirectr.sqlite`.
    pub database_url: String,
    /// Maximum number of pooled database connections. From `MAX_CONN`,
    /// defaults to `9`.
    pub max_connections: u32,
    /// URL returned as the `Location` header for `GET /`. From `DEFAULT_URL`,
    /// defaults to `https://github.com/andybzn/redirectr`.
    pub default_url: String,
    /// Bearer token required to create short links. From `ADMIN_TOKEN`, no
    /// default; [`Config::new`] errors if unset or empty.
    pub token: String,
}

impl Config {
    /// Builds a [`Config`] from environment variables.
    ///
    /// Environment variables are read when this function is called. Missing
    /// values use the defaults as listed on [`Config`]. `ADMIN_TOKEN`
    /// is required; the other settings are optional.
    ///
    /// An empty `DATABASE_URL` is treated as missing and uses its default.
    /// `MAX_CONN` falls back to `9` when it is missing or cannot be parsed as
    /// a [`u32`]. Other optional variables are used as provided when present,
    /// including empty strings.
    ///
    /// This function does not validate the format of the server address,
    /// database URL, or default URL. Those values are validated or used by
    /// the components that consume them.
    ///
    /// # Errors
    ///
    /// Returns an error when `ADMIN_TOKEN` is not set or is empty. The error is
    /// returned as a [`String`] with the message
    /// `Config Error: env_var ADMIN_TOKEN not set or empty.`.
    pub fn new() -> Result<Self, String> {
        let token: String = env::var("ADMIN_TOKEN").unwrap_or_default();
        if token.is_empty() {
            event!(
                Level::ERROR,
                "CONFIG: env_var ADMIN_TOKEN not set or empty."
            );
            return Err("Config Error: env_var ADMIN_TOKEN not set or empty.".to_string());
        }

        let mut database_url: String = env::var("DATABASE_URL").unwrap_or_default();
        if database_url.is_empty() {
            database_url = "sqlite://redirectr.sqlite".to_string();
        }

        Ok(Self {
            server_port: env::var("SERVER_PORT").unwrap_or_else(|_| "3333".to_string()),
            server_ip: env::var("SERVER_IP").unwrap_or_else(|_| "127.0.0.1".to_string()),
            database_url,
            max_connections: env::var("MAX_CONN")
                .unwrap_or_else(|_| "9".to_string())
                .parse::<u32>()
                .unwrap_or(9),
            default_url: env::var("DEFAULT_URL")
                .unwrap_or_else(|_| "https://github.com/andybzn/redirectr".to_string()),
            token,
        })
    }
}
