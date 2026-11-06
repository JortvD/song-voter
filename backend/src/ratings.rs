use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::elo;
use crate::error::ApiError;

/// A vote as needed by the replay engine.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReplayVote {
    pub id: Uuid,
    pub track_low: Uuid,
    pub track_high: Uuid,
    pub winner: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub struct TrackState {
    pub rating: f64,
    pub comparisons: i32,
    pub wins: i32,
    pub last_compared_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct RecomputedVote {
    pub id: Uuid,
    pub rating_low_before: f64,
    pub rating_high_before: f64,
    pub delta_low: f64,
    pub delta_high: f64,
    pub k_factor: f64,
    pub comparisons_low_before: i32,
    pub comparisons_high_before: i32,
}

#[derive(Debug)]
pub struct ReplayResult {
    pub tracks: HashMap<Uuid, TrackState>,
    pub votes: Vec<RecomputedVote>,
}

pub struct RebuildSummary {
    pub votes_replayed: usize,
    pub tracks_updated: usize,
}

/// Replay a playlist's votes from scratch. This is the counterfactual used
/// after deleting a voter's picks: the result is what the standings would be
/// if those picks had never happened.
pub fn replay_votes(track_ids: &[Uuid], votes: &[ReplayVote]) -> ReplayResult {
    let mut tracks = track_ids
        .iter()
        .map(|id| {
            (
                *id,
                TrackState {
                    rating: elo::INITIAL_RATING,
                    comparisons: 0,
                    wins: 0,
                    last_compared_at: None,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut recomputed = Vec::with_capacity(votes.len());

    for vote in votes {
        let Some(low_before) = tracks.get(&vote.track_low).copied() else {
            continue;
        };
        let Some(high_before) = tracks.get(&vote.track_high).copied() else {
            continue;
        };

        let low_won = vote.winner == vote.track_low;
        let outcome = elo::update(
            low_before.rating,
            high_before.rating,
            low_before.comparisons,
            high_before.comparisons,
            low_won,
        );

        let low = tracks.get_mut(&vote.track_low).expect("known low track");
        low.rating = low_before.rating + outcome.delta_low;
        low.comparisons += 1;
        low.wins += i32::from(low_won);
        low.last_compared_at = Some(vote.created_at);

        let high = tracks.get_mut(&vote.track_high).expect("known high track");
        high.rating = high_before.rating + outcome.delta_high;
        high.comparisons += 1;
        high.wins += i32::from(!low_won);
        high.last_compared_at = Some(vote.created_at);

        recomputed.push(RecomputedVote {
            id: vote.id,
            rating_low_before: low_before.rating,
            rating_high_before: high_before.rating,
            delta_low: outcome.delta_low,
            delta_high: outcome.delta_high,
            k_factor: outcome.k,
            comparisons_low_before: low_before.comparisons,
            comparisons_high_before: high_before.comparisons,
        });
    }

    ReplayResult {
        tracks,
        votes: recomputed,
    }
}

/// Rewrite a playlist's track aggregates and vote ledger from its remaining
/// votes. Locks every track in the playlist first, so no vote can land midway
/// through the replay.
pub async fn rebuild_playlist_ratings(
    connection: &mut PgConnection,
    playlist_id: Uuid,
) -> Result<RebuildSummary, ApiError> {
    let track_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM tracks WHERE playlist_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(playlist_id)
    .fetch_all(&mut *connection)
    .await?;

    let votes = sqlx::query_as::<_, ReplayVote>(
        "SELECT id, track_low, track_high, winner, created_at \
         FROM votes WHERE playlist_id = $1 \
         ORDER BY created_at ASC, id ASC",
    )
    .bind(playlist_id)
    .fetch_all(&mut *connection)
    .await?;

    let replay = replay_votes(&track_ids, &votes);

    if !track_ids.is_empty() {
        let mut ids = Vec::with_capacity(track_ids.len());
        let mut ratings = Vec::with_capacity(track_ids.len());
        let mut comparisons = Vec::with_capacity(track_ids.len());
        let mut wins = Vec::with_capacity(track_ids.len());
        let mut last_compared = Vec::with_capacity(track_ids.len());

        for id in &track_ids {
            let state = replay.tracks[id];
            ids.push(*id);
            ratings.push(state.rating);
            comparisons.push(state.comparisons);
            wins.push(state.wins);
            last_compared.push(state.last_compared_at);
        }

        sqlx::query(
            "UPDATE tracks AS t \
             SET rating = data.rating, \
                 comparisons = data.comparisons, \
                 wins = data.wins, \
                 last_compared_at = data.last_compared_at \
             FROM UNNEST($1::uuid[], $2::float8[], $3::integer[], $4::integer[], \
                         $5::timestamptz[]) \
                 AS data(id, rating, comparisons, wins, last_compared_at) \
             WHERE t.id = data.id",
        )
        .bind(&ids)
        .bind(&ratings)
        .bind(&comparisons)
        .bind(&wins)
        .bind(&last_compared)
        .execute(&mut *connection)
        .await?;
    }

    if !replay.votes.is_empty() {
        let ids = replay.votes.iter().map(|vote| vote.id).collect::<Vec<_>>();
        let rating_low_before = replay
            .votes
            .iter()
            .map(|vote| vote.rating_low_before)
            .collect::<Vec<_>>();
        let rating_high_before = replay
            .votes
            .iter()
            .map(|vote| vote.rating_high_before)
            .collect::<Vec<_>>();
        let delta_low = replay
            .votes
            .iter()
            .map(|vote| vote.delta_low)
            .collect::<Vec<_>>();
        let delta_high = replay
            .votes
            .iter()
            .map(|vote| vote.delta_high)
            .collect::<Vec<_>>();
        let k_factor = replay
            .votes
            .iter()
            .map(|vote| vote.k_factor)
            .collect::<Vec<_>>();
        let comparisons_low_before = replay
            .votes
            .iter()
            .map(|vote| vote.comparisons_low_before)
            .collect::<Vec<_>>();
        let comparisons_high_before = replay
            .votes
            .iter()
            .map(|vote| vote.comparisons_high_before)
            .collect::<Vec<_>>();

        sqlx::query(
            "UPDATE votes AS v \
             SET rating_low_before = data.rating_low_before, \
                 rating_high_before = data.rating_high_before, \
                 delta_low = data.delta_low, \
                 delta_high = data.delta_high, \
                 k_factor = data.k_factor, \
                 comparisons_low_before = data.comparisons_low_before, \
                 comparisons_high_before = data.comparisons_high_before \
             FROM UNNEST($1::uuid[], $2::float8[], $3::float8[], $4::float8[], \
                         $5::float8[], $6::float8[], $7::integer[], $8::integer[]) \
                 AS data(id, rating_low_before, rating_high_before, delta_low, \
                         delta_high, k_factor, comparisons_low_before, \
                         comparisons_high_before) \
             WHERE v.id = data.id",
        )
        .bind(&ids)
        .bind(&rating_low_before)
        .bind(&rating_high_before)
        .bind(&delta_low)
        .bind(&delta_high)
        .bind(&k_factor)
        .bind(&comparisons_low_before)
        .bind(&comparisons_high_before)
        .execute(&mut *connection)
        .await?;
    }

    Ok(RebuildSummary {
        votes_replayed: replay.votes.len(),
        tracks_updated: track_ids.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vote(id: u8, low: Uuid, high: Uuid, winner: Uuid, second: i64) -> ReplayVote {
        ReplayVote {
            id: Uuid::from_u128(id as u128),
            track_low: low,
            track_high: high,
            winner,
            created_at: DateTime::from_timestamp(second, 0).unwrap(),
        }
    }

    #[test]
    fn replay_rebuilds_track_aggregates_and_ledger() {
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let votes = vec![vote(1, low, high, low, 1), vote(2, low, high, high, 2)];

        let result = replay_votes(&[low, high], &votes);

        assert_eq!(result.votes.len(), 2);
        assert_eq!(result.tracks[&low].comparisons, 2);
        assert_eq!(result.tracks[&high].comparisons, 2);
        assert_eq!(result.tracks[&low].wins, 1);
        assert_eq!(result.tracks[&high].wins, 1);
        assert!(result.tracks[&low].rating < elo::INITIAL_RATING);
        assert!(result.tracks[&high].rating > elo::INITIAL_RATING);
        assert_eq!(result.votes[0].rating_low_before, elo::INITIAL_RATING);
        assert_ne!(
            result.votes[1].rating_low_before,
            elo::INITIAL_RATING,
            "the second vote must start from the first vote's result"
        );
    }

    #[test]
    fn replay_ignores_votes_for_missing_tracks() {
        let low = Uuid::from_u128(1);
        let high = Uuid::from_u128(2);
        let missing = Uuid::from_u128(3);
        let votes = vec![vote(1, low, missing, low, 1)];

        let result = replay_votes(&[low, high], &votes);

        assert!(result.votes.is_empty());
        assert_eq!(result.tracks[&low].comparisons, 0);
        assert_eq!(result.tracks[&high].comparisons, 0);
    }
}
