use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::chart::{load_chart, ChartQuery, PublicChart};
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyVotes {
    date: String,
    votes: i64,
}

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    playlist_id: Option<Uuid>,
    playlist_name: Option<String>,
    revealed: bool,
    total_tracks: i64,
    tracks_compared: i64,
    total_users: i64,
    active_users: i64,
    total_votes: i64,
    votes_today: i64,
    active_today: i64,
    total_matchups: i64,
    daily: Vec<DailyVotes>,
}

#[derive(Debug, sqlx::FromRow)]
struct StatsCounts {
    total_tracks: i64,
    tracks_compared: i64,
    total_users: i64,
    active_users: i64,
    total_votes: i64,
    votes_today: i64,
    active_today: i64,
    total_matchups: i64,
}

#[derive(Debug, Deserialize)]
pub struct UsersQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminUsersPage {
    total: i64,
    limit: i64,
    offset: i64,
    query: String,
    users: Vec<AdminUserRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    id: Uuid,
    username: String,
    is_admin: bool,
    created_at: DateTime<Utc>,
    votes_cast: i64,
    matchups_seen: i64,
    last_vote_at: Option<DateTime<Utc>>,
    last_seen_at: Option<DateTime<Utc>>,
    last_active_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ResetRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct ResetResponse {
    reset: bool,
    deleted_votes: u64,
    deleted_matchups: u64,
}

#[derive(Debug, Deserialize)]
pub struct RemovePlaylistRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct RemovePlaylistResponse {
    removed: bool,
    removed_tracks: i64,
    removed_votes: i64,
}

pub async fn stats(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<DashboardStats>, ApiError> {
    let playlist = active_playlist(&state.db).await?;
    let playlist_id = playlist.as_ref().map(|playlist| playlist.id);

    let counts = sqlx::query_as::<_, StatsCounts>(
        "SELECT \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1) AS total_tracks, \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1 AND comparisons > 0) AS tracks_compared, \
            (SELECT COUNT(*) FROM users) AS total_users, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1) AS active_users, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1) AS total_votes, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1 \
             AND created_at >= date_trunc('day', now())) AS votes_today, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1 \
             AND created_at >= date_trunc('day', now())) AS active_today, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1) AS total_matchups",
    )
    .bind(playlist_id)
    .fetch_one(&state.db)
    .await?;

    let daily = sqlx::query_as::<_, DailyVotes>(
        "SELECT to_char(day::date, 'YYYY-MM-DD') AS date, COUNT(v.id) AS votes \
         FROM generate_series( \
             current_date - interval '13 days', \
             current_date, \
             interval '1 day' \
         ) AS day \
         LEFT JOIN votes v \
           ON v.playlist_id = $1 \
          AND v.created_at >= day \
          AND v.created_at < day + interval '1 day' \
         GROUP BY day \
         ORDER BY day",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let (playlist_name, revealed) = match playlist {
        Some(playlist) => (
            Some(playlist.name),
            playlist.reveal_state == "revealed",
        ),
        None => (None, false),
    };

    Ok(Json(DashboardStats {
        playlist_id,
        playlist_name,
        revealed,
        total_tracks: counts.total_tracks,
        tracks_compared: counts.tracks_compared,
        total_users: counts.total_users,
        active_users: counts.active_users,
        total_votes: counts.total_votes,
        votes_today: counts.votes_today,
        active_today: counts.active_today,
        total_matchups: counts.total_matchups,
        daily,
    }))
}

pub async fn users(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<UsersQuery>,
) -> Result<Json<AdminUsersPage>, ApiError> {
    let playlist = active_playlist(&state.db).await?;
    let playlist_id = playlist.as_ref().map(|playlist| playlist.id);

    let limit = query.limit.unwrap_or(50).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users \
         WHERE ($1 = '' OR username ILIKE '%' || $1 || '%')",
    )
    .bind(&search)
    .fetch_one(&state.db)
    .await?;

    let users = sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.username, u.is_admin, u.created_at, \
                COALESCE(v.votes, 0) AS votes_cast, \
                COALESCE(s.matchups, 0) AS matchups_seen, \
                v.last_vote_at, \
                s.last_seen_at, \
                COALESCE(v.last_vote_at, s.last_seen_at, u.created_at) AS last_active_at \
         FROM users u \
         LEFT JOIN LATERAL ( \
             SELECT COUNT(*) AS votes, MAX(created_at) AS last_vote_at \
             FROM votes \
             WHERE user_id = u.id \
               AND ($1::uuid IS NULL OR playlist_id = $1) \
         ) v ON TRUE \
         LEFT JOIN LATERAL ( \
             SELECT COUNT(*) AS matchups, MAX(seen_at) AS last_seen_at \
             FROM seen_pairs \
             WHERE user_id = u.id \
               AND ($1::uuid IS NULL OR playlist_id = $1) \
         ) s ON TRUE \
         WHERE ($2 = '' OR u.username ILIKE '%' || $2 || '%') \
         ORDER BY COALESCE(v.last_vote_at, s.last_seen_at, u.created_at) DESC, \
                  u.created_at DESC \
         LIMIT $3 OFFSET $4",
    )
    .bind(playlist_id)
    .bind(&search)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(AdminUsersPage {
        total,
        limit,
        offset,
        query: search,
        users,
    }))
}

/// The shared ranking as seen by the admin, regardless of reveal state.
pub async fn ranking(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<ChartQuery>,
) -> Result<Json<PublicChart>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    Ok(Json(
        load_chart(&state.db, &playlist, limit, offset, &search).await?,
    ))
}

pub async fn reset_votes(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<ResetRequest>,
) -> Result<Json<ResetResponse>, ApiError> {
    if payload.confirm != "RESET" {
        return Err(ApiError::bad_request(
            "type RESET to confirm this destructive action",
        ));
    }

    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let mut transaction = state.db.begin().await?;

    let deleted_votes = sqlx::query("DELETE FROM votes WHERE playlist_id = $1")
        .bind(playlist.id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    let deleted_matchups = sqlx::query("DELETE FROM seen_pairs WHERE playlist_id = $1")
        .bind(playlist.id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    sqlx::query(
        "UPDATE tracks \
         SET rating = 1000, comparisons = 0, wins = 0, last_compared_at = NULL \
         WHERE playlist_id = $1",
    )
    .bind(playlist.id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("UPDATE playlists SET reveal_state = 'hidden' WHERE id = $1")
        .bind(playlist.id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    tracing::warn!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.id,
        deleted_votes,
        deleted_matchups,
        "all votes reset"
    );

    Ok(Json(ResetResponse {
        reset: true,
        deleted_votes,
        deleted_matchups,
    }))
}

pub async fn remove_playlist(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<RemovePlaylistRequest>,
) -> Result<Json<RemovePlaylistResponse>, ApiError> {
    if payload.confirm != "REMOVE" {
        return Err(ApiError::bad_request(
            "type REMOVE to confirm removing the playlist",
        ));
    }

    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let mut transaction = state.db.begin().await?;

    let removed_tracks =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&mut *transaction)
            .await?;

    let removed_votes =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM votes WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&mut *transaction)
            .await?;

    // Tracks, seen pairs and votes cascade with the playlist.
    sqlx::query("DELETE FROM playlists WHERE id = $1")
        .bind(playlist.id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;

    tracing::warn!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.id,
        removed_tracks,
        removed_votes,
        "playlist removed"
    );

    Ok(Json(RemovePlaylistResponse {
        removed: true,
        removed_tracks,
        removed_votes,
    }))
}
