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

/// Best-effort authentication for public endpoints that add personal context
/// when a valid session is present. Missing or invalid sessions are treated as
/// anonymous; database failures still surface as server errors.
pub struct OptionalAuthUser(pub Option<User>);

impl FromRequestParts<AppState> for OptionalAuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        if jar.get(SESSION_COOKIE).is_none() {
            return Ok(Self(None));
        }

        match AuthUser::from_request_parts(parts, state).await {
            Ok(AuthUser(user)) => Ok(Self(Some(user))),
            Err(ApiError::Unauthorized(_)) => Ok(Self(None)),
            Err(error) => Err(error),
        }
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
    if password.chars().count() > 128 {
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
                .and_then(|value| value.split(',').next_back())
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

    fn test_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.into(),
            password_hash: "unused".into(),
            is_admin: false,
            created_at: Utc::now(),
            disabled_at: None,
            session_version: 7,
        }
    }

    #[test]
    fn username_accepts_minimum_and_maximum_lengths() {
        assert_eq!(validate_username("abc").unwrap(), "abc");
        assert_eq!(validate_username(&"a".repeat(32)).unwrap(), "a".repeat(32));
    }

    #[test]
    fn username_rejects_one_character_too_long() {
        assert!(validate_username(&"a".repeat(33)).is_err());
    }

    #[test]
    fn username_counts_characters_not_bytes() {
        // 33 multi-byte characters must still be rejected even though the byte
        // length is much larger.
        assert!(validate_username(&"é".repeat(33)).is_err());
    }

    #[test]
    fn username_rejects_non_ascii_characters() {
        for name in ["josé", "名字", "rock🎸", "space\ttab", "line\nbreak"] {
            assert!(validate_username(name).is_err(), "{name:?} should fail");
        }
    }

    #[test]
    fn username_allows_underscores_and_dashes() {
        assert_eq!(validate_username(" dj_jort-2000 ").unwrap(), "dj_jort-2000");
    }

    #[test]
    fn username_rejects_empty_and_whitespace_only() {
        assert!(validate_username("").is_err());
        assert!(validate_username("   ").is_err());
    }

    #[test]
    fn password_accepts_exactly_eight_characters() {
        assert!(validate_password("12345678").is_ok());
    }

    #[test]
    fn password_rejects_seven_characters() {
        assert!(validate_password("1234567").is_err());
    }

    #[test]
    fn password_accepts_multibyte_characters_by_character_count() {
        assert!(validate_password("éééééééé").is_ok());
    }

    #[test]
    fn password_rejects_more_than_128_characters() {
        assert!(validate_password(&"a".repeat(129)).is_err());
        assert!(validate_password(&"a".repeat(128)).is_ok());
        assert!(validate_password(&"é".repeat(129)).is_err());
        assert!(validate_password(&"é".repeat(128)).is_ok());
    }

    #[test]
    fn password_does_not_trim_whitespace() {
        assert!(validate_password("        ").is_ok());
        assert!(validate_password("short   ").is_ok());
        assert!(validate_password("       ").is_err());
    }

    #[tokio::test]
    async fn verify_password_rejects_a_malformed_hash() {
        assert!(verify_password("anything", "not-a-hash").await.is_err());
    }

    #[tokio::test]
    async fn verifies_passwords_with_unicode_and_spaces() {
        let password = "  señor contraseña  ";
        let hash = hash_password(password).await.unwrap();
        assert!(verify_password(password, &hash).await.unwrap());
        assert!(!verify_password("señor contraseña", &hash).await.unwrap());
    }

    #[test]
    fn jwt_carries_the_session_version_and_admin_flag() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        let mut user = test_user("voter");
        user.is_admin = true;
        let claims = service
            .decode_token(&service.issue_token(&user).unwrap())
            .unwrap();
        assert_eq!(claims.session_version, 7);
        assert!(claims.is_admin);
    }

    #[test]
    fn jwt_is_rejected_with_a_different_secret() {
        let issuer = AuthService::new("first-secret-at-least-32-characters", 1, false);
        let verifier = AuthService::new("second-secret-at-least-32-characters", 1, false);
        let token = issuer.issue_token(&test_user("voter")).unwrap();
        assert!(verifier.decode_token(&token).is_err());
    }

    #[test]
    fn jwt_is_rejected_after_tampering() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        let token = service.issue_token(&test_user("voter")).unwrap();
        let mut bytes = token.into_bytes();
        let index = bytes.len() / 2;
        bytes[index] = if bytes[index] == b'a' { b'b' } else { b'a' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(service.decode_token(&tampered).is_err());
    }

    #[test]
    fn expired_jwt_is_rejected() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", -1, false);
        let token = service.issue_token(&test_user("voter")).unwrap();
        assert!(service.decode_token(&token).is_err());
    }

    #[test]
    fn malformed_jwt_is_rejected() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        for token in ["", "not.a.token", "a.b", "...."] {
            assert!(service.decode_token(token).is_err(), "{token:?}");
        }
    }

    #[test]
    fn legacy_claims_without_session_version_default_to_zero() {
        #[derive(Serialize)]
        struct LegacyClaims<'a> {
            sub: Uuid,
            username: &'a str,
            is_admin: bool,
            iat: i64,
            exp: i64,
        }

        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        let now = Utc::now().timestamp();
        let token = encode(
            &Header::default(),
            &LegacyClaims {
                sub: Uuid::new_v4(),
                username: "legacy",
                is_admin: false,
                iat: now,
                exp: now + 60,
            },
            &EncodingKey::from_secret(b"test-secret-at-least-32-characters-long"),
        )
        .unwrap();

        let claims = service.decode_token(&token).unwrap();
        assert_eq!(claims.session_version, 0);
    }

    #[test]
    fn session_cookie_is_http_only_and_lax() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 2, true);
        let cookie = service.session_cookie("token-value".to_string());
        assert_eq!(cookie.name(), SESSION_COOKIE);
        assert_eq!(cookie.value(), "token-value");
        assert_eq!(cookie.path(), Some("/"));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.same_site(), Some(SameSite::Lax));
        assert_eq!(cookie.max_age(), Some(cookie::time::Duration::hours(2)));
    }

    #[test]
    fn session_cookie_respects_insecure_development_setting() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 1, false);
        let cookie = service.session_cookie("token".to_string());
        assert_eq!(cookie.secure(), Some(false));
    }

    #[test]
    fn removal_cookie_expires_immediately() {
        let service = AuthService::new("test-secret-at-least-32-characters-long", 2, true);
        let cookie = service.removal_cookie();
        assert_eq!(cookie.name(), SESSION_COOKIE);
        assert_eq!(cookie.value(), "");
        assert_eq!(cookie.max_age(), Some(cookie::time::Duration::ZERO));
        assert_eq!(cookie.path(), Some("/"));
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        map
    }

    #[test]
    fn client_info_prefers_x_real_ip() {
        let map = headers(&[
            ("x-real-ip", "203.0.113.9"),
            ("x-forwarded-for", "198.51.100.1, 203.0.113.9"),
            ("user-agent", "TestBrowser/1.0"),
        ]);
        let (ip, agent) = client_info(&map);
        assert_eq!(ip.as_deref(), Some("203.0.113.9"));
        assert_eq!(agent.as_deref(), Some("TestBrowser/1.0"));
    }

    #[test]
    fn client_info_uses_the_last_forwarded_address() {
        let map = headers(&[(
            "x-forwarded-for",
            " 198.51.100.1 , 203.0.113.7 , 192.0.2.4 ",
        )]);
        let (ip, _) = client_info(&map);
        assert_eq!(ip.as_deref(), Some("192.0.2.4"));
    }

    #[test]
    fn client_info_ignores_empty_proxy_headers() {
        let map = headers(&[("x-real-ip", "   "), ("x-forwarded-for", " , 203.0.113.2")]);
        let (ip, _) = client_info(&map);
        assert_eq!(ip.as_deref(), Some("203.0.113.2"));
    }

    #[test]
    fn client_info_returns_none_without_headers() {
        let (ip, agent) = client_info(&HeaderMap::new());
        assert_eq!(ip, None);
        assert_eq!(agent, None);
    }

    #[test]
    fn client_info_truncates_to_database_bounds() {
        let long_ip = "1".repeat(200);
        let long_agent = "u".repeat(2_000);
        let map = headers(&[("x-real-ip", &long_ip), ("user-agent", &long_agent)]);
        let (ip, agent) = client_info(&map);
        assert_eq!(ip.unwrap().chars().count(), 64);
        assert_eq!(agent.unwrap().chars().count(), 512);
    }

    #[test]
    fn truncate_counts_characters_not_bytes() {
        assert_eq!(truncate("ééé", 2), "éé");
        assert_eq!(truncate("abc", 10), "abc");
        assert_eq!(truncate("abc", 0), "");
    }
}
