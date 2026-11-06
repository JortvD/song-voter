use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::audit::{self, AuthEvent};
use crate::auth::{client_info, hash_password, validate_password, validate_username, AdminUser};
use crate::error::{is_unique_violation, ApiError};
use crate::ratings;
use crate::state::AppState;

#[derive(Debug, sqlx::FromRow)]
struct TargetUser {
    id: Uuid,
    username: String,
    is_admin: bool,
    disabled_at: Option<DateTime<Utc>>,
}

/// Serializes admin-count-sensitive mutations so two admins cannot delete or
/// disable each other simultaneously and leave the app without an active admin.
const USER_MANAGEMENT_LOCK: i64 = 0x7076_5f75_7365_7273;

#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteUserResponse {
    deleted: bool,
    id: Uuid,
    username: String,
    deleted_votes: i64,
    deleted_duels: i64,
    rebuilt_playlists: usize,
    votes_replayed: usize,
    tracks_updated: usize,
}

#[derive(Debug, Deserialize)]
pub struct ClearVotesRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct ClearVotesResponse {
    cleared: bool,
    id: Uuid,
    username: String,
    playlist_id: Uuid,
    playlist_name: String,
    deleted_votes: i64,
    deleted_duels: i64,
    votes_replayed: usize,
    tracks_updated: usize,
}

#[derive(Debug, Deserialize)]
pub struct RenameUserRequest {
    username: String,
}

#[derive(Debug, Serialize)]
pub struct RenameUserResponse {
    updated: bool,
    id: Uuid,
    username: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    password: String,
}

#[derive(Debug, Serialize)]
pub struct ResetPasswordResponse {
    updated: bool,
    id: Uuid,
    username: String,
    signed_out: bool,
}

#[derive(Debug, Deserialize)]
pub struct SetDisabledRequest {
    disabled: bool,
}

#[derive(Debug, Serialize)]
pub struct SetDisabledResponse {
    updated: bool,
    id: Uuid,
    username: String,
    disabled: bool,
    signed_out: bool,
}

#[derive(Debug, Serialize)]
pub struct ForceLogoutResponse {
    signed_out: bool,
    id: Uuid,
    username: String,
}

pub async fn delete_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<DeleteUserRequest>,
) -> Result<Json<DeleteUserResponse>, ApiError> {
    let (ip, user_agent) = client_info(&headers);
    let mut transaction = state.db.begin().await?;
    lock_user_management(&mut *transaction).await?;
    let target = lock_user(&mut *transaction, id).await?;

    if target.id == admin.0.id {
        transaction.rollback().await.ok();
        return Err(ApiError::bad_request("you cannot delete your own account"));
    }

    if payload.confirm.trim() != target.username {
        transaction.rollback().await.ok();
        return Err(ApiError::bad_request(
            "type the voter's username to confirm deleting this account",
        ));
    }

    if target.is_admin
        && target.disabled_at.is_none()
        && enabled_admin_count(&mut *transaction).await? <= 1
    {
        transaction.rollback().await.ok();
        return Err(ApiError::conflict(
            "the last active admin cannot be deleted",
        ));
    }

    let playlist_ids =
        sqlx::query_scalar::<_, Uuid>("SELECT DISTINCT playlist_id FROM votes WHERE user_id = $1")
            .bind(target.id)
            .fetch_all(&mut *transaction)
            .await?;

    let deleted_votes =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM votes WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(&mut *transaction)
            .await?;

    let deleted_duels =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM seen_pairs WHERE user_id = $1")
            .bind(target.id)
            .fetch_one(&mut *transaction)
            .await?;

    // Votes, seen pairs and other references cascade or become null.
    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(target.id)
        .execute(&mut *transaction)
        .await?;

    let mut votes_replayed = 0;
    let mut tracks_updated = 0;

    for playlist_id in &playlist_ids {
        let summary = ratings::rebuild_playlist_ratings(&mut *transaction, *playlist_id).await?;
        votes_replayed += summary.votes_replayed;
        tracks_updated += summary.tracks_updated;
    }

    transaction.commit().await?;
    state.auth_cache.invalidate(target.id);
    state.cache.clear();

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_user_deleted",
            detail: Some(&target.username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        deleted_user_id = %target.id,
        username = %target.username,
        deleted_votes,
        deleted_duels,
        rebuilt_playlists = playlist_ids.len(),
        "admin deleted voter"
    );

    Ok(Json(DeleteUserResponse {
        deleted: true,
        id: target.id,
        username: target.username,
        deleted_votes,
        deleted_duels,
        rebuilt_playlists: playlist_ids.len(),
        votes_replayed,
        tracks_updated,
    }))
}

pub async fn clear_votes(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<ClearVotesRequest>,
) -> Result<Json<ClearVotesResponse>, ApiError> {
    if payload.confirm.trim() != "CLEAR" {
        return Err(ApiError::bad_request(
            "type CLEAR to confirm removing picks",
        ));
    }

    let (ip, user_agent) = client_info(&headers);
    let mut transaction = state.db.begin().await?;
    let target = lock_user(&mut *transaction, id).await?;

    let playlist = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM playlists WHERE is_active LIMIT 1",
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("no active playlist has been imported yet".to_string()))?;

    let deleted_votes = sqlx::query("DELETE FROM votes WHERE user_id = $1 AND playlist_id = $2")
        .bind(target.id)
        .bind(playlist.0)
        .execute(&mut *transaction)
        .await?
        .rows_affected() as i64;

    // Seen pairs are the voter's per-playlist coverage. Removing only their
    // votes would leave that coverage behind and, because matchups skip every
    // pair a voter has already seen, permanently hide those duels from them.
    let deleted_duels =
        sqlx::query("DELETE FROM seen_pairs WHERE user_id = $1 AND playlist_id = $2")
            .bind(target.id)
            .bind(playlist.0)
            .execute(&mut *transaction)
            .await?
            .rows_affected() as i64;

    let summary = if deleted_votes > 0 {
        ratings::rebuild_playlist_ratings(&mut *transaction, playlist.0).await?
    } else {
        ratings::RebuildSummary {
            votes_replayed: 0,
            tracks_updated: 0,
        }
    };

    transaction.commit().await?;
    state.cache.clear();

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_votes_cleared",
            detail: Some(&target.username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        user_id = %target.id,
        username = %target.username,
        playlist_id = %playlist.0,
        deleted_votes,
        deleted_duels,
        "admin cleared voter picks"
    );

    Ok(Json(ClearVotesResponse {
        cleared: true,
        id: target.id,
        username: target.username,
        playlist_id: playlist.0,
        playlist_name: playlist.1,
        deleted_votes,
        deleted_duels,
        votes_replayed: summary.votes_replayed,
        tracks_updated: summary.tracks_updated,
    }))
}

pub async fn rename_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<RenameUserRequest>,
) -> Result<Json<RenameUserResponse>, ApiError> {
    let username = validate_username(&payload.username)?;
    let (ip, user_agent) = client_info(&headers);

    let updated = sqlx::query_scalar::<_, String>(
        "UPDATE users SET username = $1 WHERE id = $2 RETURNING username",
    )
    .bind(&username)
    .bind(id)
    .fetch_optional(&state.db)
    .await;

    let username = match updated {
        Ok(Some(username)) => username,
        Ok(None) => return Err(ApiError::NotFound("voter not found".to_string())),
        Err(error) if is_unique_violation(&error) => {
            return Err(ApiError::conflict("username is already taken"));
        }
        Err(error) => return Err(error.into()),
    };

    state.auth_cache.invalidate(id);
    state.cache.clear();

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_user_renamed",
            detail: Some(&username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::info!(
        admin_id = %admin.0.id,
        user_id = %id,
        username = %username,
        "admin renamed voter"
    );

    Ok(Json(RenameUserResponse {
        updated: true,
        id,
        username,
    }))
}

pub async fn reset_password(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<ResetPasswordRequest>,
) -> Result<Json<ResetPasswordResponse>, ApiError> {
    validate_password(&payload.password)?;
    let password_hash = hash_password(&payload.password).await?;
    let (ip, user_agent) = client_info(&headers);

    let updated = sqlx::query_scalar::<_, String>(
        "UPDATE users SET password_hash = $1, session_version = session_version + 1 \
         WHERE id = $2 RETURNING username",
    )
    .bind(&password_hash)
    .bind(id)
    .fetch_optional(&state.db)
    .await?;

    let Some(username) = updated else {
        return Err(ApiError::NotFound("voter not found".to_string()));
    };

    state.auth_cache.invalidate(id);

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_password_reset",
            detail: Some(&username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        user_id = %id,
        username = %username,
        "admin reset voter password"
    );

    Ok(Json(ResetPasswordResponse {
        updated: true,
        id,
        username,
        signed_out: true,
    }))
}

pub async fn set_disabled(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<SetDisabledRequest>,
) -> Result<Json<SetDisabledResponse>, ApiError> {
    let (ip, user_agent) = client_info(&headers);
    let mut transaction = state.db.begin().await?;
    lock_user_management(&mut *transaction).await?;
    let target = lock_user(&mut *transaction, id).await?;

    if payload.disabled && target.id == admin.0.id {
        transaction.rollback().await.ok();
        return Err(ApiError::bad_request("you cannot disable your own account"));
    }

    if payload.disabled
        && target.is_admin
        && target.disabled_at.is_none()
        && enabled_admin_count(&mut *transaction).await? <= 1
    {
        transaction.rollback().await.ok();
        return Err(ApiError::conflict(
            "the last active admin cannot be disabled",
        ));
    }

    let username = sqlx::query_scalar::<_, String>(
        "UPDATE users \
         SET disabled_at = CASE WHEN $1 THEN now() ELSE NULL END, \
             session_version = session_version + CASE WHEN $1 THEN 1 ELSE 0 END \
         WHERE id = $2 RETURNING username",
    )
    .bind(payload.disabled)
    .bind(target.id)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;
    state.auth_cache.invalidate(target.id);

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: if payload.disabled {
                "admin_user_disabled"
            } else {
                "admin_user_enabled"
            },
            detail: Some(&username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        user_id = %id,
        username = %username,
        disabled = payload.disabled,
        "admin changed voter account status"
    );

    Ok(Json(SetDisabledResponse {
        updated: true,
        id,
        username,
        disabled: payload.disabled,
        signed_out: payload.disabled,
    }))
}

pub async fn force_logout(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<ForceLogoutResponse>, ApiError> {
    let (ip, user_agent) = client_info(&headers);

    let username = sqlx::query_scalar::<_, String>(
        "UPDATE users SET session_version = session_version + 1 \
         WHERE id = $1 RETURNING username",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("voter not found".to_string()))?;

    state.auth_cache.invalidate(id);

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_sessions_revoked",
            detail: Some(&username),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        user_id = %id,
        username = %username,
        "admin revoked voter sessions"
    );

    Ok(Json(ForceLogoutResponse {
        signed_out: true,
        id,
        username,
    }))
}

async fn lock_user(connection: &mut PgConnection, id: Uuid) -> Result<TargetUser, ApiError> {
    sqlx::query_as::<_, TargetUser>(
        "SELECT id, username, is_admin, disabled_at FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| ApiError::NotFound("voter not found".to_string()))
}

async fn lock_user_management(connection: &mut PgConnection) -> Result<(), ApiError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(USER_MANAGEMENT_LOCK)
        .execute(connection)
        .await?;

    Ok(())
}

async fn enabled_admin_count(connection: &mut PgConnection) -> Result<i64, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE is_admin AND disabled_at IS NULL",
    )
    .fetch_one(connection)
    .await?;

    Ok(count)
}
