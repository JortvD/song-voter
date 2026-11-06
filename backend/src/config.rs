use std::env;

use anyhow::Context;
use uuid::Uuid;

use crate::spotify::{
    DEFAULT_ACCOUNTS_BASE, DEFAULT_API_BASE, DEFAULT_AUTHORIZE_URL, DEFAULT_EMBED_BASE,
};

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
        let environment = env::var("APP_ENV")
            .unwrap_or_else(|_| "development".into())
            .trim()
            .to_ascii_lowercase();
        let production = environment == "production";

        let database_url =
            env::var("DATABASE_URL").context("DATABASE_URL must be set (see .env.example)")?;

        if production && looks_like_placeholder(&database_url) {
            anyhow::bail!(
                "DATABASE_URL still contains an example password; set POSTGRES_PASSWORD in .env \
                 to a strong value before running in production"
            );
        }

        let (jwt_secret, jwt_secret_was_generated) = match non_empty("JWT_SECRET") {
            Some(secret) => (secret, false),
            None => (Uuid::new_v4().simple().to_string(), true),
        };

        if production {
            anyhow::ensure!(
                !jwt_secret_was_generated,
                "JWT_SECRET must be set in production; generate one with `openssl rand -hex 32`"
            );
            anyhow::ensure!(
                !looks_like_placeholder(&jwt_secret),
                "JWT_SECRET still contains the example placeholder; generate a fresh one with \
                 `openssl rand -hex 32`"
            );
            anyhow::ensure!(
                jwt_secret.len() >= 32,
                "JWT_SECRET must be at least 32 characters in production"
            );
        }

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

        let admin_username = non_empty("ADMIN_USERNAME");
        let admin_password = non_empty("ADMIN_PASSWORD");

        if production {
            if let Some(password) = admin_password.as_deref() {
                anyhow::ensure!(
                    !looks_like_placeholder(password) && password != "admin",
                    "ADMIN_PASSWORD still uses an example or weak value; set a strong password \
                     before running in production"
                );
            }
        }

        Ok(Self {
            environment,
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".into())
                .parse()
                .context("PORT must be a valid u16")?,
            database_url,
            log_filter: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            frontend_dir: env::var("FRONTEND_DIR").unwrap_or_else(|_| "./frontend/dist".into()),
            jwt_secret,
            jwt_secret_was_generated,
            jwt_ttl_hours,
            cookie_secure,
            login_rate_limit,
            register_rate_limit,
            admin_username,
            admin_password,
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

/// Reject the values shipped in `.env.example` so a copied file cannot silently
/// run production with a known password or signing key.
fn looks_like_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();

    lower.contains("change-me")
        || lower.contains("changeme")
        || lower.contains("replace-with")
        || lower.contains("local-dev")
        || lower.contains(":ulltop2000@")
        || lower == "ulltop2000"
        || lower == "admin"
        || lower == "password"
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => anyhow::bail!("unrecognised boolean {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_placeholder;

    #[test]
    fn rejects_example_and_weak_values() {
        assert!(looks_like_placeholder("change-me"));
        assert!(looks_like_placeholder("CHANGE-ME-run-openssl-rand-hex-32"));
        assert!(looks_like_placeholder(
            "postgres://user:change-me@localhost/app"
        ));
        assert!(looks_like_placeholder(
            "postgres://user:ulltop2000@localhost/app"
        ));
        assert!(looks_like_placeholder("admin"));
        assert!(looks_like_placeholder("password"));
    }

    #[test]
    fn accepts_random_production_values() {
        assert!(!looks_like_placeholder(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!looks_like_placeholder("a-long-random-admin-password"));
    }
}
