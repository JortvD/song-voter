use std::env;

use anyhow::Context;
use uuid::Uuid;

use crate::spotify::{DEFAULT_ACCOUNTS_BASE, DEFAULT_API_BASE, DEFAULT_AUTHORIZE_URL, DEFAULT_EMBED_BASE};

/// Runtime configuration, entirely sourced from environment variables.
#[derive(Clone, Debug)]
pub struct Config {
    pub environment: String,
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub log_filter: String,
    pub frontend_dir: String,
    pub jwt_secret: String,
    pub jwt_secret_was_generated: bool,
    pub jwt_ttl_hours: i64,
    pub cookie_secure: bool,
    pub login_rate_limit: u32,
    pub register_rate_limit: u32,
    pub admin_username: Option<String>,
    pub admin_password: Option<String>,
    pub spotify_client_id: Option<String>,
    pub spotify_client_secret: Option<String>,
    pub spotify_api_base: String,
    pub spotify_accounts_base: String,
    pub spotify_authorize_url: String,
    pub spotify_embed_base: String,
    pub spotify_redirect_uri: Option<String>,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let environment = env::var("APP_ENV").unwrap_or_else(|_| "development".into());

        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL must be set (see .env.example)")?;

        let (jwt_secret, jwt_secret_was_generated) = match non_empty("JWT_SECRET") {
            Some(secret) => (secret, false),
            None => (Uuid::new_v4().simple().to_string(), true),
        };

        let jwt_ttl_hours = match non_empty("JWT_TTL_HOURS") {
            Some(value) => value
                .parse()
                .context("JWT_TTL_HOURS must be a positive integer")?,
            None => 168,
        };
        anyhow::ensure!(jwt_ttl_hours > 0, "JWT_TTL_HOURS must be positive");

        let cookie_secure = match non_empty("COOKIE_SECURE") {
            Some(value) => parse_bool(&value).context("COOKIE_SECURE must be true or false")?,
            None => environment == "production",
        };

        let login_rate_limit = match non_empty("AUTH_LOGIN_RATE_LIMIT") {
            Some(value) => value
                .parse()
                .context("AUTH_LOGIN_RATE_LIMIT must be a positive integer")?,
            None => 30,
        };
        anyhow::ensure!(
            login_rate_limit > 0,
            "AUTH_LOGIN_RATE_LIMIT must be positive"
        );

        let register_rate_limit = match non_empty("AUTH_REGISTER_RATE_LIMIT") {
            Some(value) => value
                .parse()
                .context("AUTH_REGISTER_RATE_LIMIT must be a positive integer")?,
            None => 40,
        };
        anyhow::ensure!(
            register_rate_limit > 0,
            "AUTH_REGISTER_RATE_LIMIT must be positive"
        );

        Ok(Self {
            environment,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .context("PORT must be a valid u16")?,
            database_url,
            log_filter: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            frontend_dir: env::var("FRONTEND_DIR")
                .unwrap_or_else(|_| "./frontend/dist".into()),
            jwt_secret,
            jwt_secret_was_generated,
            jwt_ttl_hours,
            cookie_secure,
            login_rate_limit,
            register_rate_limit,
            admin_username: non_empty("ADMIN_USERNAME"),
            admin_password: non_empty("ADMIN_PASSWORD"),
            spotify_client_id: non_empty("SPOTIFY_CLIENT_ID"),
            spotify_client_secret: non_empty("SPOTIFY_CLIENT_SECRET"),
            spotify_api_base: non_empty("SPOTIFY_API_BASE")
                .unwrap_or_else(|| DEFAULT_API_BASE.to_string()),
            spotify_accounts_base: non_empty("SPOTIFY_ACCOUNTS_BASE")
                .unwrap_or_else(|| DEFAULT_ACCOUNTS_BASE.to_string()),
            spotify_authorize_url: non_empty("SPOTIFY_AUTHORIZE_URL")
                .unwrap_or_else(|| DEFAULT_AUTHORIZE_URL.to_string()),
            spotify_embed_base: non_empty("SPOTIFY_EMBED_BASE")
                .unwrap_or_else(|| DEFAULT_EMBED_BASE.to_string()),
            spotify_redirect_uri: non_empty("SPOTIFY_REDIRECT_URI"),
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

fn non_empty(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => anyhow::bail!("unrecognised boolean {other:?}"),
    }
}
