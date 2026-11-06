use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::api::history::{recent_picked, DuelHistoryItem, RECENT_DUEL_LIMIT};
use crate::api::streak::{load_streak, StreakQuery, StreakSummary};
use crate::auth::AuthUser;
use crate::db::{user_progress, voting_block_reason_parts};
use crate::elo;
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    track_a: Uuid,
    track_b: Uuid,
    winner: Uuid,
}

#[derive(Debug, Serialize)]
pub struct VoteResponse {
    vote_id: Uuid,
    playlist_id: Uuid,
    track_a: Uuid,
    track_b: Uuid,
    winner: Uuid,
    created_at: DateTime<Utc>,
    votes_cast: i64,
}

#[derive(Debug, Serialize)]
pub struct UndoResponse {
    undone_vote_id: Uuid,
    playlist_id: Uuid,
    track_a: MatchupTrack,
    track_b: MatchupTrack,
    winner: Uuid,
    created_at: DateTime<Utc>,
    compared_pairs: i64,
    tracks_seen: i64,
    votes_cast: i64,
    recent_duels: Vec<DuelHistoryItem>,
    streak: StreakSummary,
}

#[derive(Debug, FromRow)]
struct VoteTrack {
    rating: f64,
    comparisons: i32,
}

#[derive(Debug, FromRow)]
struct InsertedVote {
    id: Uuid,
    created_at: DateTime<Utc>,
}

async fn ensure_voting_allowed(
    connection: &mut PgConnection,
    playlist_id: Uuid,
) -> Result<(), ApiError> {
    let (started_at, scheduled_start_at, voting_paused) =
        sqlx::query_as::<_, (Option<DateTime<Utc>>, Option<DateTime<Utc>>, bool)>(
            "SELECT started_at, scheduled_start_at, voting_paused \
             FROM playlists WHERE id = $1",
        )
        .bind(playlist_id)
        .fetch_one(connection)
        .await?;

    if let Some(reason) = voting_block_reason_parts(started_at, scheduled_start_at, voting_paused) {
        return Err(ApiError::Conflict(reason));
    }

    Ok(())
}

#[derive(Debug, FromRow)]
struct UndoableVote {
    id: Uuid,
    track_low: Uuid,
    track_high: Uuid,
    winner: Uuid,
    delta_low: f64,
    delta_high: f64,
    created_at: DateTime<Utc>,
}

/// Record a pick on a duel that was previously served to this user.
pub async fn cast(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<VoteRequest>,
) -> Result<Json<VoteResponse>, ApiError> {
    if payload.track_a == payload.track_b {
        return Err(ApiError::bad_request("a duel needs two different tracks"));
    }
    if payload.winner != payload.track_a && payload.winner != payload.track_b {
        return Err(ApiError::bad_request(
            "winner must be one of the two tracks",
        ));
    }

    let (low_id, high_id) = if payload.track_a < payload.track_b {
        (payload.track_a, payload.track_b)
    } else {
        (payload.track_b, payload.track_a)
    };

    let mut transaction = state.db.begin().await?;

    // The pair must have been served to this user in the currently active
    // playlist. This stops arbitrary or stale pair voting.
    let playlist_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT sp.playlist_id \
         FROM seen_pairs sp \
         JOIN playlists p ON p.id = sp.playlist_id AND p.is_active \
         WHERE sp.user_id = $1 AND sp.track_low = $2 AND sp.track_high = $3 \
         LIMIT 1",
    )
    .bind(user.0.id)
    .bind(low_id)
    .bind(high_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let Some(playlist_id) = playlist_id else {
        transaction.rollback().await.ok();
        tracing::warn!(
            user_id = %user.0.id,
            track_low = %low_id,
            track_high = %high_id,
            "pick rejected: duel was not served or playlist is not active"
        );
        return Err(ApiError::conflict(
            "this duel was not served to you, or the playlist changed",
        ));
    };

    ensure_voting_allowed(&mut transaction, playlist_id).await?;

    // Lock both tracks in a consistent (UUID) order to avoid deadlocks.
    let low = lock_track(&mut transaction, low_id, playlist_id).await?;
    let high = lock_track(&mut transaction, high_id, playlist_id).await?;

    let already_voted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM votes \
         WHERE user_id = $1 AND playlist_id = $2 AND track_low = $3 AND track_high = $4)",
    )
    .bind(user.0.id)
    .bind(playlist_id)
    .bind(low_id)
    .bind(high_id)
    .fetch_one(&mut *transaction)
    .await?;

    if already_voted {
        transaction.rollback().await.ok();
        tracing::warn!(
            user_id = %user.0.id,
            track_low = %low_id,
            track_high = %high_id,
            "pick rejected: duel already decided"
        );
        return Err(ApiError::conflict("you already picked a side in this duel"));
    }

    let low_won = payload.winner == low_id;
    let outcome = elo::update(
        low.rating,
        high.rating,
        low.comparisons,
        high.comparisons,
        low_won,
    );

    let new_low_rating = low.rating + outcome.delta_low;
    let new_high_rating = high.rating + outcome.delta_high;

    sqlx::query(
        "UPDATE tracks SET rating = $1, comparisons = comparisons + 1, \
                          wins = wins + $2, last_compared_at = now() WHERE id = $3",
    )
    .bind(new_low_rating)
    .bind(i32::from(low_won))
    .bind(low_id)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE tracks SET rating = $1, comparisons = comparisons + 1, \
                          wins = wins + $2, last_compared_at = now() WHERE id = $3",
    )
    .bind(new_high_rating)
    .bind(i32::from(!low_won))
    .bind(high_id)
    .execute(&mut *transaction)
    .await?;

    let inserted = sqlx::query_as::<_, InsertedVote>(
        "INSERT INTO votes \
            (user_id, playlist_id, track_low, track_high, winner, \
             rating_low_before, rating_high_before, delta_low, delta_high, k_factor, \
             comparisons_low_before, comparisons_high_before) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
         RETURNING id, created_at",
    )
    .bind(user.0.id)
    .bind(playlist_id)
    .bind(low_id)
    .bind(high_id)
    .bind(payload.winner)
    .bind(low.rating)
    .bind(high.rating)
    .bind(outcome.delta_low)
    .bind(outcome.delta_high)
    .bind(outcome.k)
    .bind(low.comparisons)
    .bind(high.comparisons)
    .fetch_one(&mut *transaction)
    .await?;

    let votes_cast = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM votes WHERE user_id = $1 AND playlist_id = $2",
    )
    .bind(user.0.id)
    .bind(playlist_id)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;

    tracing::info!(
        user_id = %user.0.id,
        playlist_id = %playlist_id,
        track_low = %low_id,
        track_high = %high_id,
        winner = %payload.winner,
        k = %outcome.k,
        expected_low = %outcome.expected_low,
        delta_low = %outcome.delta_low,
        delta_high = %outcome.delta_high,
        "vote recorded"
    );

    Ok(Json(VoteResponse {
        vote_id: inserted.id,
        playlist_id,
        track_a: payload.track_a,
        track_b: payload.track_b,
        winner: payload.winner,
        created_at: inserted.created_at,
        votes_cast,
    }))
}

/// Reverse this user's most recent vote in the active playlist.
pub async fn undo(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<StreakQuery>,
) -> Result<Json<UndoResponse>, ApiError> {
    let mut transaction = state.db.begin().await?;

    let playlist_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM playlists WHERE is_active LIMIT 1")
            .fetch_optional(&mut *transaction)
            .await?;

    let Some(playlist_id) = playlist_id else {
        transaction.rollback().await.ok();
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    ensure_voting_allowed(&mut transaction, playlist_id).await?;

    let vote = sqlx::query_as::<_, UndoableVote>(
        "SELECT id, track_low, track_high, winner, delta_low, delta_high, created_at \
         FROM votes \
         WHERE user_id = $1 AND playlist_id = $2 \
         ORDER BY created_at DESC, id DESC LIMIT 1 \
         FOR UPDATE",
    )
    .bind(user.0.id)
    .bind(playlist_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let Some(vote) = vote else {
        transaction.rollback().await.ok();
        return Err(ApiError::NotFound("you have no pick to undo".to_string()));
    };

    // Lock both tracks before changing them, in the same order as voting.
    lock_track(&mut transaction, vote.track_low, playlist_id).await?;
    lock_track(&mut transaction, vote.track_high, playlist_id).await?;

    // Subtract exactly this vote's deltas. Other users' votes stay intact.
    sqlx::query(
        "UPDATE tracks SET rating = rating - $1, \
                          comparisons = GREATEST(comparisons - 1, 0), \
                          wins = GREATEST(wins - $2, 0) \
         WHERE id = $3",
    )
    .bind(vote.delta_low)
    .bind(i32::from(vote.winner == vote.track_low))
    .bind(vote.track_low)
    .execute(&mut *transaction)
    .await?;

    sqlx::query(
        "UPDATE tracks SET rating = rating - $1, \
                          comparisons = GREATEST(comparisons - 1, 0), \
                          wins = GREATEST(wins - $2, 0) \
         WHERE id = $3",
    )
    .bind(vote.delta_high)
    .bind(i32::from(vote.winner == vote.track_high))
    .bind(vote.track_high)
    .execute(&mut *transaction)
    .await?;

    sqlx::query("DELETE FROM votes WHERE id = $1")
        .bind(vote.id)
        .execute(&mut *transaction)
        .await?;

    let tracks = sqlx::query_as::<_, MatchupTrack>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, duration_ms, \
                release_year, decade, album_id, album_type, total_tracks, disc_number, \
                track_number, primary_genre \
         FROM tracks WHERE id = ANY($1)",
    )
    .bind(vec![vote.track_low, vote.track_high])
    .fetch_all(&mut *transaction)
    .await?;

    let low = tracks
        .iter()
        .find(|track| track.id == vote.track_low)
        .cloned()
        .ok_or_else(|| ApiError::internal(anyhow::anyhow!("undone low track is missing")))?;
    let high = tracks
        .iter()
        .find(|track| track.id == vote.track_high)
        .cloned()
        .ok_or_else(|| ApiError::internal(anyhow::anyhow!("undone high track is missing")))?;

    let progress = user_progress(&mut *transaction, user.0.id, playlist_id).await?;
    let recent_duels =
        recent_picked(&mut *transaction, user.0.id, playlist_id, RECENT_DUEL_LIMIT).await?;
    let streak = load_streak(&mut *transaction, user.0.id, query.tz.as_deref()).await?;

    transaction.commit().await?;

    tracing::info!(
        user_id = %user.0.id,
        playlist_id = %playlist_id,
        vote_id = %vote.id,
        track_low = %vote.track_low,
        track_high = %vote.track_high,
        winner = %vote.winner,
        delta_low = %vote.delta_low,
        delta_high = %vote.delta_high,
        votes_cast = progress.votes_cast,
        compared_pairs = progress.compared_pairs,
        "vote undone"
    );

    Ok(Json(UndoResponse {
        undone_vote_id: vote.id,
        playlist_id,
        track_a: low,
        track_b: high,
        winner: vote.winner,
        created_at: vote.created_at,
        compared_pairs: progress.compared_pairs,
        tracks_seen: progress.tracks_seen,
        votes_cast: progress.votes_cast,
        recent_duels,
        streak,
    }))
}

async fn lock_track(
    connection: &mut PgConnection,
    id: Uuid,
    playlist_id: Uuid,
) -> Result<VoteTrack, ApiError> {
    let track = sqlx::query_as::<_, VoteTrack>(
        "SELECT rating, comparisons FROM tracks \
         WHERE id = $1 AND playlist_id = $2 FOR UPDATE",
    )
    .bind(id)
    .bind(playlist_id)
    .fetch_optional(&mut *connection)
    .await?;

    track.ok_or_else(|| {
        ApiError::conflict("a track in this duel is no longer in the active playlist")
    })
}
