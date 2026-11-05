use axum::extract::State;
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::{active_playlist, user_progress};
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

/// How many of the least-compared tracks are considered for a new matchup.
const CANDIDATE_POOL: i64 = 100;
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
) -> Result<Json<MatchupResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

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
            .fetch_optional(&mut *transaction)
            .await?;

        let Some(pair) = pair else {
            transaction.rollback().await.ok();
            tracing::debug!(
                user_id = %user.0.id,
                playlist_id = %playlist.id,
                "no unseen matchup available"
            );
            return Err(ApiError::Conflict(
                "you have compared every possible pair in this playlist".to_string(),
            ));
        };

        let inserted = sqlx::query(
            "INSERT INTO seen_pairs (user_id, playlist_id, track_low, track_high) \
             VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
        )
        .bind(user.0.id)
        .bind(playlist.id)
        .bind(pair.low_id)
        .bind(pair.high_id)
        .execute(&mut *transaction)
        .await?;

        if inserted.rows_affected() == 0 {
            transaction.rollback().await.ok();
            tracing::warn!(
                attempt,
                user_id = %user.0.id,
                playlist_id = %playlist.id,
                "matchup pair was recorded concurrently, retrying"
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
            .ok_or_else(|| {
                ApiError::internal(anyhow::anyhow!("selected low track is missing"))
            })?;
        let high = tracks
            .iter()
            .find(|track| track.id == pair.high_id)
            .cloned()
            .ok_or_else(|| {
                ApiError::internal(anyhow::anyhow!("selected high track is missing"))
            })?;

        let progress = user_progress(&mut *transaction, user.0.id, playlist.id).await?;

        transaction.commit().await?;

        // Randomise which track appears on the left so position gives no hint
        // about the internal (low/high) UUID ordering.
        let (track_a, track_b) = if pair.flip {
            (high, low)
        } else {
            (low, high)
        };

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
            "matchup served"
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
        }));
    }

    tracing::warn!(
        user_id = %user.0.id,
        playlist_id = %playlist.id,
        "failed to lock in a fresh matchup after retries"
    );
    Err(ApiError::conflict(
        "could not lock in a fresh matchup; please try again",
    ))
}

const PAIR_SQL: &str = r#"
WITH candidates AS (
    SELECT id, rating
    FROM tracks
    WHERE playlist_id = $1
    ORDER BY comparisons ASC, last_compared_at ASC NULLS LAST, random()
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
SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, duration_ms
FROM tracks
WHERE id = ANY($1)
"#;
