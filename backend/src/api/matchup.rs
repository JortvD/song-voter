use axum::extract::{Query, State};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::api::history::{recent_picked, DuelHistoryItem, RECENT_DUEL_LIMIT};
use crate::api::streak::{load_streak, StreakQuery, StreakSummary};
use crate::auth::AuthUser;
use crate::db::{active_playlist, user_progress, voting_block_reason};
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

/// How many of the least-compared tracks are considered for a new matchup.
/// 40 keeps the pair search at 780 combinations instead of 4,950 for a
/// hundred-track pool, while still giving every voter plenty of variety.
const CANDIDATE_POOL: i64 = 40;
/// Randomise within a slightly wider slice so not every voter sees exactly
/// the same small group of tracks.
const CANDIDATE_SAMPLE: i64 = 200;
const MAX_INSERT_ATTEMPTS: u32 = 3;

#[derive(Debug, Serialize)]
pub struct MatchupResponse {
    playlist_id: Uuid,
    playlist_name: String,
    track_a: MatchupTrack,
    track_b: MatchupTrack,
    compared_pairs: i64,
    tracks_seen: i64,
    total_tracks: i64,
    votes_cast: i64,
    recent_duels: Vec<DuelHistoryItem>,
    streak: StreakSummary,
}

#[derive(Debug, sqlx::FromRow)]
struct PairRow {
    low_id: Uuid,
    high_id: Uuid,
    flip: bool,
}

/// Pick the next pair for the current user.
///
/// Strategy:
/// 1. Take the 100 least-compared tracks (ties broken by least recently compared).
/// 2. Among those, prefer the pair with the smallest Elo difference.
/// 3. Never repeat a pair the user has already seen.
pub async fn next(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<StreakQuery>,
) -> Result<Json<MatchupResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    if let Some(reason) = voting_block_reason(&playlist) {
        return Err(ApiError::Conflict(reason));
    }

    let total_tracks =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&state.db)
            .await?;

    if total_tracks < 2 {
        return Err(ApiError::Conflict(
            "the playlist needs at least two tracks".to_string(),
        ));
    }

    for attempt in 1..=MAX_INSERT_ATTEMPTS {
        let mut transaction = state.db.begin().await?;

        let pair = sqlx::query_as::<_, PairRow>(PAIR_SQL)
            .bind(playlist.id)
            .bind(user.0.id)
            .bind(CANDIDATE_POOL)
            .bind(CANDIDATE_SAMPLE)
            .fetch_optional(&mut *transaction)
            .await?;

        let Some(pair) = pair else {
            transaction.rollback().await.ok();
            tracing::debug!(
                user_id = %user.0.id,
                playlist_id = %playlist.id,
                "no unseen duel available"
            );
            return Err(ApiError::Conflict(
                "you have seen every possible duel in this playlist".to_string(),
            ));
        };

        let inserted = sqlx::query(
            "INSERT INTO seen_pairs \
                (user_id, playlist_id, track_low, track_high, served_flip) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        )
        .bind(user.0.id)
        .bind(playlist.id)
        .bind(pair.low_id)
        .bind(pair.high_id)
        .bind(pair.flip)
        .execute(&mut *transaction)
        .await?;

        if inserted.rows_affected() == 0 {
            transaction.rollback().await.ok();
            tracing::warn!(
                attempt,
                user_id = %user.0.id,
                playlist_id = %playlist.id,
                "duel pair was recorded concurrently, retrying"
            );
            continue;
        }

        let tracks = sqlx::query_as::<_, MatchupTrack>(TRACK_SQL)
            .bind(vec![pair.low_id, pair.high_id])
            .fetch_all(&mut *transaction)
            .await?;

        let low = tracks
            .iter()
            .find(|track| track.id == pair.low_id)
            .cloned()
            .ok_or_else(|| ApiError::internal(anyhow::anyhow!("selected low track is missing")))?;
        let high = tracks
            .iter()
            .find(|track| track.id == pair.high_id)
            .cloned()
            .ok_or_else(|| ApiError::internal(anyhow::anyhow!("selected high track is missing")))?;

        let progress = user_progress(&mut transaction, user.0.id, playlist.id).await?;
        let recent_duels =
            recent_picked(&mut transaction, user.0.id, playlist.id, RECENT_DUEL_LIMIT).await?;
        let streak = load_streak(&mut transaction, user.0.id, query.tz.as_deref()).await?;

        transaction.commit().await?;

        // Randomise which track appears on the left so position gives no hint
        // about the internal (low/high) UUID ordering.
        let (track_a, track_b) = if pair.flip { (high, low) } else { (low, high) };

        tracing::debug!(
            user_id = %user.0.id,
            playlist_id = %playlist.id,
            track_a = %track_a.id,
            track_b = %track_b.id,
            compared_pairs = progress.compared_pairs,
            tracks_seen = progress.tracks_seen,
            votes_cast = progress.votes_cast,
            total_tracks,
            attempt,
            "duel served"
        );

        return Ok(Json(MatchupResponse {
            playlist_id: playlist.id,
            playlist_name: playlist.name,
            track_a,
            track_b,
            compared_pairs: progress.compared_pairs,
            tracks_seen: progress.tracks_seen,
            total_tracks,
            votes_cast: progress.votes_cast,
            recent_duels,
            streak,
        }));
    }

    tracing::warn!(
        user_id = %user.0.id,
        playlist_id = %playlist.id,
        "failed to start a fresh duel after retries"
    );
    Err(ApiError::conflict(
        "could not start a fresh duel; please try again",
    ))
}

const PAIR_SQL: &str = r#"
WITH least_compared AS (
    SELECT id, rating
    FROM tracks
    WHERE playlist_id = $1
    ORDER BY comparisons ASC, last_compared_at ASC NULLS LAST
    LIMIT $4
),
candidates AS (
    SELECT id, rating
    FROM least_compared
    ORDER BY random()
    LIMIT $3
),
unseen AS (
    SELECT
        a.id AS low_id,
        b.id AS high_id,
        abs(a.rating - b.rating) AS rating_gap,
        random() < 0.5 AS flip
    FROM candidates a
    JOIN candidates b ON a.id < b.id
    WHERE NOT EXISTS (
        SELECT 1
        FROM seen_pairs sp
        WHERE sp.user_id = $2
          AND sp.playlist_id = $1
          AND sp.track_low = a.id
          AND sp.track_high = b.id
    )
)
SELECT low_id, high_id, flip
FROM unseen
ORDER BY rating_gap ASC, random()
LIMIT 1
"#;

const TRACK_SQL: &str = r#"
SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, duration_ms,
       release_year, decade, album_id, album_type, total_tracks, disc_number,
       track_number, primary_genre
FROM tracks
WHERE id = ANY($1)
"#;
