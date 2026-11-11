//! Admin-only track maintenance.
//!
//! Removing a track is destructive: its votes and seen pairs are deleted and
//! the playlist's Elo ledger is rebuilt from the remaining votes. This is meant
//! for mistakes such as a duplicate entry, not for hiding a song from the vote.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::audit::{self, AuthEvent};
use crate::auth::{client_info, AdminUser};
use crate::error::ApiError;
use crate::ratings;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RemoveTrackRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveTrackResponse {
    removed: bool,
    id: Uuid,
    title: String,
    artists: String,
    rank: i64,
    deleted_votes: i64,
    deleted_duels: i64,
    votes_replayed: usize,
    tracks_updated: usize,
    remaining_tracks: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct TrackRow {
    id: Uuid,
    title: String,
    artists: String,
    rank: i64,
    total_tracks: i64,
}

/// Remove one track from the active playlist.
///
/// Votes and seen pairs cascade with the track row. The remaining ledger is
/// then replayed so every other track's rating, comparisons and wins reflect a
/// vote without the removed song.
pub async fn remove_track(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(track_id): Path<Uuid>,
    headers: HeaderMap,
    Json(payload): Json<RemoveTrackRequest>,
) -> Result<Json<RemoveTrackResponse>, ApiError> {
    if payload.confirm.trim() != "REMOVE" {
        return Err(ApiError::bad_request(
            "type REMOVE to confirm deleting this track and its vote history",
        ));
    }

    let _permit = state
        .heavy_work
        .acquire()
        .await
        .map_err(|_| ApiError::service_unavailable("the server is busy; please try again"))?;

    let (ip, user_agent) = client_info(&headers);
    let mut transaction = state.db.begin().await?;

    let playlist = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM playlists WHERE is_active LIMIT 1 FOR UPDATE",
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("no active playlist has been imported yet".to_string()))?;

    let track = sqlx::query_as::<_, TrackRow>(
        "WITH ranked AS ( \
             SELECT id, title, artists, \
                    ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) AS rank, \
                    COUNT(*) OVER () AS total_tracks \
             FROM tracks WHERE playlist_id = $1 \
         ) \
         SELECT id, title, artists, rank, total_tracks FROM ranked WHERE id = $2",
    )
    .bind(playlist.0)
    .bind(track_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("track not found in the active playlist".to_string()))?;

    if track.total_tracks <= 2 {
        transaction.rollback().await.ok();
        return Err(ApiError::conflict(
            "a playlist needs at least two tracks for duels",
        ));
    }

    let deleted_votes = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM votes \
         WHERE playlist_id = $1 AND (track_low = $2 OR track_high = $2)",
    )
    .bind(playlist.0)
    .bind(track.id)
    .fetch_one(&mut *transaction)
    .await?;

    let deleted_duels = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM seen_pairs \
         WHERE playlist_id = $1 AND (track_low = $2 OR track_high = $2)",
    )
    .bind(playlist.0)
    .bind(track.id)
    .fetch_one(&mut *transaction)
    .await?;

    // Votes, seen pairs and track_artists references cascade with the row.
    let deleted = sqlx::query("DELETE FROM tracks WHERE id = $1 AND playlist_id = $2")
        .bind(track.id)
        .bind(playlist.0)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    if deleted == 0 {
        transaction.rollback().await.ok();
        return Err(ApiError::NotFound(
            "track disappeared before it could be removed".to_string(),
        ));
    }

    let summary = ratings::rebuild_playlist_ratings(&mut transaction, playlist.0).await?;
    let remaining_tracks = track.total_tracks - 1;

    transaction.commit().await?;
    state.cache.clear();

    audit::record(
        &state.db,
        AuthEvent {
            user_id: Some(admin.0.id),
            username: Some(&admin.0.username),
            event: "admin_track_removed",
            detail: Some(&format!(
                "{}|{}|rank {}|{} votes|{} duels",
                track.title, track.artists, track.rank, deleted_votes, deleted_duels
            )),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
        },
    )
    .await;

    tracing::warn!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.0,
        track_id = %track.id,
        title = %track.title,
        rank = track.rank,
        deleted_votes,
        deleted_duels,
        votes_replayed = summary.votes_replayed,
        tracks_updated = summary.tracks_updated,
        "admin removed track and rebuilt ratings"
    );

    Ok(Json(RemoveTrackResponse {
        removed: true,
        id: track.id,
        title: track.title,
        artists: track.artists,
        rank: track.rank,
        deleted_votes,
        deleted_duels,
        votes_replayed: summary.votes_replayed,
        tracks_updated: summary.tracks_updated,
        remaining_tracks,
    }))
}
