use std::time::Duration;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::audit::{self, AuthEvent};
use crate::auth::{
    burn_password_time, client_info, hash_password, validate_password, validate_username,
    verify_password, AuthUser, SESSION_COOKIE,
};
use crate::error::{is_unique_violation, ApiError};
use crate::models::User;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
    invite_code: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let username = validate_username(&payload.username)?;
    validate_password(&payload.password)?;

    let invite_code = payload.invite_code.trim().to_ascii_uppercase();
    if invite_code.is_empty() {
        return Err(ApiError::bad_request("invite code is required"));
    }

    let (ip, user_agent) = client_info(&headers);

    let rate_key = format!("register:{}", ip.as_deref().unwrap_or("unknown"));
    if !state.rate_limiter.allow(
        &rate_key,
        state.register_rate_limit,
        Duration::from_secs(3600),
    ) {
        tracing::warn!(
            ip = ip.as_deref().unwrap_or("unknown"),
            "registration rate limit exceeded"
        );
        return Err(ApiError::too_many_requests(
            "too many registration attempts; try again later",
        ));
    }

    let password_hash = hash_password(&payload.password).await?;

    let mut transaction = state.db.begin().await?;

    let invite_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM invites \
         WHERE code = $1 AND used_by IS NULL AND (expires_at IS NULL OR expires_at > now()) \
         FOR UPDATE",
    )
    .bind(&invite_code)
    .fetch_optional(&mut *transaction)
    .await?;

    let Some(invite_id) = invite_id else {
        transaction.rollback().await.ok();
        audit::record(
            &state.db,
            AuthEvent {
                user_id: None,
                username: Some(&username),
                event: "register_failed",
                detail: Some("invalid_invite"),
                ip: ip.as_deref(),
                user_agent: user_agent.as_deref(),
            },
        )
        .await;
        tracing::warn!(
            username = %username,
            ip = ip.as_deref().unwrap_or("unknown"),
            "registration rejected: invalid invite code"
        );
        return Err(ApiError::bad_request(
            "invite code is invalid, expired, or already used",
        ));
    };

    let inserted = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash) VALUES ($1, $2) \
         RETURNING id, username, password_hash, is_admin, created_at, disabled_at, \
                   session_version",
    )
    .bind(&username)
    .bind(&password_hash)
    .fetch_one(&mut *transaction)
    .await;

    let user = match inserted {
        Ok(user) => user,
        Err(error) if is_unique_violation(&error) => {
            transaction.rollback().await.ok();
            audit::record(
                &state.db,
                AuthEvent {
                    user_id: None,
                    username: Some(&username),
                    event: "register_failed",
                    detail: Some("username_taken"),
                    ip: ip.as_deref(),
                    user_agent: user_agent.as_deref(),
                },
            )
            .await;
            tracing::warn!(username = %username, "registration rejected: username taken");
            return Err(ApiError::conflict("username is already taken"));
        }
        Err(error) => return Err(error.into()),
    };

    sqlx::query("UPDATE invites SET used_by = $1, used_at = now() WHERE id = $2")
        .bind(user.id)
        .bind(invite_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(user.id),
            username: Some(&user.username),
            event: "register",
            detail: Some("invite"),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        ip = ip.as_deref().unwrap_or("unknown"),
        "user registered"
    );

    let token = state.auth.issue_token(&user)?;
    Ok((
        CookieJar::new().add(state.auth.session_cookie(token)),
        Json(user),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let username_input = payload.username.trim().to_owned();
    if username_input.is_empty() || payload.password.is_empty() {
        return Err(ApiError::bad_request("username and password are required"));
    }

    let (ip, user_agent) = client_info(&headers);

    let rate_key = format!("login:{}", ip.as_deref().unwrap_or("unknown"));
    if !state
        .rate_limiter
        .allow(&rate_key, state.login_rate_limit, Duration::from_secs(60))
    {
        tracing::warn!(
            ip = ip.as_deref().unwrap_or("unknown"),
            "login rate limit exceeded"
        );
        return Err(ApiError::too_many_requests(
            "too many login attempts; wait a minute and try again",
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, is_admin, created_at, disabled_at, \
                session_version \
         FROM users WHERE lower(username) = lower($1)",
    )
    .bind(&username_input)
    .fetch_optional(&state.db)
    .await?;

    let Some(user) = user else {
        burn_password_time().await;
        audit::record(
            &state.db,
            AuthEvent {
                user_id: None,
                username: Some(&username_input),
                event: "login_failed",
                detail: Some("unknown_user"),
                ip: ip.as_deref(),
                user_agent: user_agent.as_deref(),
            },
        )
        .await;
        tracing::warn!(
            username = %username_input,
            reason = "unknown_user",
            ip = ip.as_deref().unwrap_or("unknown"),
            "login failed"
        );
        return Err(ApiError::unauthorized("invalid username or password"));
    };

    if !verify_password(&payload.password, &user.password_hash).await? {
        audit::record(
            &state.db,
            AuthEvent {
                user_id: Some(user.id),
                username: Some(&user.username),
                event: "login_failed",
                detail: Some("bad_password"),
                ip: ip.as_deref(),
                user_agent: user_agent.as_deref(),
            },
        )
        .await;
        tracing::warn!(
            user_id = %user.id,
            username = %user.username,
            reason = "bad_password",
            ip = ip.as_deref().unwrap_or("unknown"),
            "login failed"
        );
        return Err(ApiError::unauthorized("invalid username or password"));
    }

    if user.disabled_at.is_some() {
        audit::record(
            &state.db,
            AuthEvent {
                user_id: Some(user.id),
                username: Some(&user.username),
                event: "login_failed",
                detail: Some("account_disabled"),
                ip: ip.as_deref(),
                user_agent: user_agent.as_deref(),
            },
        )
        .await;
        tracing::warn!(
            user_id = %user.id,
            username = %user.username,
            reason = "account_disabled",
            ip = ip.as_deref().unwrap_or("unknown"),
            "login failed"
        );
        return Err(ApiError::forbidden("this account has been disabled"));
    }

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(user.id),
            username: Some(&user.username),
            event: "login",
            detail: None,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        ip = ip.as_deref().unwrap_or("unknown"),
        "login successful"
    );

    let token = state.auth.issue_token(&user)?;
    Ok((
        CookieJar::new().add(state.auth.session_cookie(token)),
        Json(user),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
) -> impl IntoResponse {
    let (ip, user_agent) = client_info(&headers);
    let claims = jar
        .get(SESSION_COOKIE)
        .and_then(|cookie| state.auth.decode_token(cookie.value()).ok());

    let (user_id, username) = match claims {
        Some(claims) => (Some(claims.sub), Some(claims.username)),
        None => (None, None),
    };

    audit::record(
        &state.db,
        AuthEvent {
            user_id,
            username: username.as_deref(),
            event: "logout",
            detail: None,
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    if let Some(username) = &username {
        tracing::info!(
            user_id = ?user_id,
            username = %username,
            "user logged out"
        );
    } else {
        tracing::info!("logout requested without a valid session");
    }

    jar.remove(state.auth.removal_cookie())
}

pub async fn me(user: AuthUser) -> Json<User> {
    Json(user.0)
}
