use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::active_playlist;
use crate::elo;
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PersonalRankingResponse {
    playlist_id: Uuid,
    playlist_name: String,
    total: usize,
    votes: usize,
    limit: i64,
    offset: i64,
    tracks: Vec<RankedTrack>,
}

#[derive(Debug, Serialize)]
pub struct RankedTrack {
    rank: usize,
    track: MatchupTrack,
    rating: f64,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct PersonalVote {
    track_low: Uuid,
    track_high: Uuid,
    winner: Uuid,
}

/// Replays only this user's votes to produce their personal Elo chart.
/// The global ranking stays hidden until the reveal.
pub async fn personal(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<RankingQuery>,
) -> Result<Json<PersonalRankingResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let started = Instant::now();

    let tracks = sqlx::query_as::<_, MatchupTrack>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, duration_ms \
         FROM tracks WHERE playlist_id = $1 ORDER BY id",
    )
    .bind(playlist.id)
    .fetch_all(&state.db)
    .await?;

    let votes = sqlx::query_as::<_, PersonalVote>(
        "SELECT track_low, track_high, winner FROM votes \
         WHERE user_id = $1 AND playlist_id = $2 \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(user.0.id)
    .bind(playlist.id)
    .fetch_all(&state.db)
    .await?;

    let by_id: HashMap<Uuid, MatchupTrack> =
        tracks.into_iter().map(|track| (track.id, track)).collect();

    let mut ratings: HashMap<Uuid, f64> = by_id
        .keys()
        .map(|id| (*id, elo::INITIAL_RATING))
        .collect();
    let mut comparisons: HashMap<Uuid, i32> = by_id.keys().map(|id| (*id, 0)).collect();
    let mut wins: HashMap<Uuid, i32> = by_id.keys().map(|id| (*id, 0)).collect();

    for vote in &votes {
        if !by_id.contains_key(&vote.track_low) || !by_id.contains_key(&vote.track_high) {
            continue;
        }

        let low_won = vote.winner == vote.track_low;
        let outcome = elo::update(
            ratings[&vote.track_low],
            ratings[&vote.track_high],
            comparisons[&vote.track_low],
            comparisons[&vote.track_high],
            low_won,
        );

        *ratings.get_mut(&vote.track_low).expect("known track") += outcome.delta_low;
        *ratings.get_mut(&vote.track_high).expect("known track") += outcome.delta_high;
        *comparisons.get_mut(&vote.track_low).expect("known track") += 1;
        *comparisons.get_mut(&vote.track_high).expect("known track") += 1;
        *wins.get_mut(&vote.winner).expect("known winner") += 1;
    }

    let mut ranked: Vec<RankedTrack> = by_id
        .into_values()
        .filter_map(|track| {
            let track_comparisons = comparisons[&track.id];
            if track_comparisons == 0 {
                return None;
            }

            Some(RankedTrack {
                rank: 0,
                rating: ratings[&track.id],
                comparisons: track_comparisons,
                wins: wins[&track.id],
                track,
            })
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .rating
            .partial_cmp(&left.rating)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.wins.cmp(&left.wins))
            .then_with(|| left.track.title.cmp(&right.track.title))
    });

    let total = ranked.len();
    for (index, entry) in ranked.iter_mut().enumerate() {
        entry.rank = index + 1;
    }

    let page: Vec<RankedTrack> = ranked
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(0))
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    tracing::debug!(
        user_id = %user.0.id,
        playlist_id = %playlist.id,
        votes = votes.len(),
        ranked_tracks = total,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "personal ranking computed"
    );

    Ok(Json(PersonalRankingResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        total,
        votes: votes.len(),
        limit,
        offset,
        tracks: page,
    }))
}
