use anyhow::Context;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::header::USER_AGENT;
use axum::http::request::Parts;
use axum::http::HeaderMap;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::{Duration as ChronoDuration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::ApiError;
use crate::models::User;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "pv_session";

/// JWT signing/verification plus session-cookie construction.
#[derive(Clone)]
pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    ttl_hours: i64,
    cookie_secure: bool,
}

impl AuthService {
    pub fn new(secret: &str, ttl_hours: i64, cookie_secure: bool) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            ttl_hours,
            cookie_secure,
        }
    }

    pub fn issue_token(&self, user: &User) -> anyhow::Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: user.id,
            username: user.username.clone(),
            is_admin: user.is_admin,
            session_version: user.session_version,
            iat: now.timestamp(),
            exp: (now + ChronoDuration::hours(self.ttl_hours)).timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).context("failed to sign JWT")
    }

    pub fn decode_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let mut validation = Validation::default();
        validation.leeway = 10;
        decode::<Claims>(token, &self.decoding_key, &validation).map(|data| data.claims)
    }

    pub fn session_cookie(&self, token: String) -> Cookie<'static> {
        Cookie::build((SESSION_COOKIE, token))
            .path("/")
            .http_only(true)
            .secure(self.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(cookie::time::Duration::hours(self.ttl_hours))
            .build()
    }

    pub fn removal_cookie(&self) -> Cookie<'static> {
        Cookie::build((SESSION_COOKIE, ""))
            .path("/")
            .http_only(true)
            .secure(self.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(cookie::time::Duration::ZERO)
            .build()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub is_admin: bool,
    /// Missing in tokens issued before session revocation existed.
    #[serde(default)]
    pub session_version: i64,
    pub iat: i64,
    pub exp: i64,
}

/// Authenticated user.
///
/// The user row is cached for a very short time to keep high-volume voting
/// paths off the database. Every admin action that disables an account or
/// revokes sessions invalidates the entry, so revocation still takes effect
/// on the next request in this process.
pub struct AuthUser(pub User);

const AUTH_USER_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(token) = jar
            .get(SESSION_COOKIE)
            .map(|cookie| cookie.value().to_owned())
        else {
            return Err(ApiError::unauthorized("not logged in"));
        };

        let claims = state.auth.decode_token(&token).map_err(|error| {
            tracing::warn!(%error, "rejected session token");
            ApiError::unauthorized("invalid or expired session")
        })?;

        let cached = state.auth_cache.get(claims.sub);
        let cache_hit = cached.is_some();
        let user = match cached {
            Some(user) => user,
            None => sqlx::query_as::<_, User>(
                // The session extractor never needs the password hash.
                "SELECT id, username, ''::text AS password_hash, is_admin, created_at, \
                        disabled_at, session_version \
                 FROM users WHERE id = $1",
            )
            .bind(claims.sub)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| {
                tracing::warn!(user_id = %claims.sub, "session refers to a missing user");
                ApiError::unauthorized("invalid or expired session")
            })?,
        };

        // Check even cache hits: the row could have changed outside this
        // process, and the check is cheap.
        if user.disabled_at.is_some() {
            tracing::warn!(user_id = %user.id, "session belongs to a disabled account");
            return Err(ApiError::unauthorized("this account has been disabled"));
        }

        if claims.session_version != user.session_version {
            tracing::warn!(
                user_id = %user.id,
                token_version = claims.session_version,
                current_version = user.session_version,
                "session has been revoked"
            );
            return Err(ApiError::unauthorized(
                "this session has been signed out; sign in again",
            ));
        }

        if !cache_hit {
            state.auth_cache.insert(user.clone(), AUTH_USER_CACHE_TTL);
        }

        Ok(Self(user))
    }
}

/// Same as [`AuthUser`], but rejects non-admins.
pub struct AdminUser(pub User);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let AuthUser(user) = AuthUser::from_request_parts(parts, state).await?;

        if !user.is_admin {
            tracing::warn!(
                user_id = %user.id,
                username = %user.username,
                "admin access denied"
            );
            return Err(ApiError::forbidden("admin access required"));
        }

        Ok(Self(user))
    }
}

pub fn validate_username(raw: &str) -> Result<String, ApiError> {
    let username = raw.trim();
    let length = username.chars().count();

    if !(3..=32).contains(&length) {
        return Err(ApiError::bad_request(
            "username must be between 3 and 32 characters",
        ));
    }

    if !username
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '-')
    {
        return Err(ApiError::bad_request(
            "username may only contain letters, digits, underscores and dashes",
        ));
    }

    Ok(username.to_owned())
}

pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.chars().count() < 8 {
        return Err(ApiError::bad_request(
            "password must be at least 8 characters",
        ));
    }
    if password.len() > 128 {
        return Err(ApiError::bad_request(
            "password must be at most 128 characters",
        ));
    }

    Ok(())
}

pub async fn hash_password(password: &str) -> anyhow::Result<String> {
    let password = password.to_owned();

    tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow::anyhow!("failed to hash password: {error}"))
    })
    .await
    .context("password hashing task panicked")?
}

pub async fn verify_password(password: &str, password_hash: &str) -> anyhow::Result<bool> {
    let password = password.to_owned();
    let password_hash = password_hash.to_owned();

    tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
        let parsed = PasswordHash::new(&password_hash)
            .map_err(|error| anyhow::anyhow!("invalid password hash in database: {error}"))?;

        match Argon2::default().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(error) => Err(anyhow::anyhow!("password verification failed: {error}")),
        }
    })
    .await
    .context("password verification task panicked")?
}

/// Equalises rough response time for unknown usernames, to reduce enumeration.
pub async fn burn_password_time() {
    let _ = hash_password("this-is-not-a-real-password").await;
}

/// Extracts a best-effort client IP and user agent for the audit trail.
///
/// `X-Real-IP` is preferred because nginx sets it to `$remote_addr`. If it is
/// absent, the last `X-Forwarded-For` entry is used (the address appended by our
/// proxy), which avoids trusting a client-supplied first value.
pub fn client_info(headers: &HeaderMap) -> (Option<String>, Option<String>) {
    let ip = headers
        .get("x-real-ip")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| truncate(value, 64))
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').last())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| truncate(value, 64))
        });

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| truncate(value, 512));

    (ip, user_agent)
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn username_validation_accepts_normal_names() {
        assert_eq!(validate_username("  jort_92 ").unwrap(), "jort_92");
    }

    #[test]
    fn username_validation_rejects_short_and_odd_names() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("bad name").is_err());
        assert!(validate_username("semi;colon").is_err());
    }

    #[test]
    fn password_validation_enforces_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough").is_ok());
    }

    #[tokio::test]
    async fn passwords_hash_and_verify() {
        let hash = hash_password("correct horse battery staple").await.unwrap();
        assert!(verify_password("correct horse battery staple", &hash)
            .await
            .unwrap());
        assert!(!verify_password("wrong password", &hash).await.unwrap());
    }

    #[test]
    fn jwt_round_trip_preserves_identity() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        let user = User {
            id: Uuid::new_v4(),
            username: "jort".into(),
            password_hash: "unused".into(),
            is_admin: true,
            created_at: Utc::now(),
            disabled_at: None,
            session_version: 0,
        };

        let token = service.issue_token(&user).unwrap();
        let claims = service.decode_token(&token).unwrap();

        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, "jort");
        assert!(claims.is_admin);
    }
}
