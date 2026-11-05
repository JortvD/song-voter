//! Volume-adjusted ("balanced") ranking.
//!
//! The official ranking consumes every pick equally, so a voter with a thousand
//! picks has far more influence than a voter with twenty. The balanced ranking
//! replays each voter's own picks into a personal Elo score, then combines those
//! personal scores with three controls:
//!
//! * influence grows with `picks^exponent` (0.5 = square root by default),
//! * a personal score is shrunk toward neutral by how often that voter compared
//!   the song (`comparisons / (comparisons + confidence_prior)`),
//! * songs with little aggregate evidence are pulled toward 1000 by
//!   `neutral_prior`.
//!
//! All three knobs, plus an optional minimum pick count, live in the singleton
//! `balanced_settings` table and are editable from the admin dashboard.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::elo;
use crate::error::ApiError;
use crate::models::{MatchupTrack, Playlist};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct BalancedQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    q: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BalancedSettings {
    pub influence_exponent: f64,
    pub confidence_prior: f64,
    pub neutral_prior: f64,
    pub min_picks: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BalancedSettingsInput {
    pub influence_exponent: f64,
    pub confidence_prior: f64,
    pub neutral_prior: f64,
    pub min_picks: i32,
}

#[derive(Debug, Serialize)]
pub struct BalancedChart {
    pub playlist_id: Uuid,
    pub playlist_name: String,
    pub revealed: bool,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub query: String,
    pub tracks: Vec<BalancedEntry>,
}

#[derive(Debug, Serialize)]
pub struct BalancedEntry {
    /// `None` when no voter has compared the song yet.
    pub rank: Option<usize>,
    pub track: MatchupTrack,
    /// `None` when the song has no balanced data.
    pub rating: Option<f64>,
    pub comparisons: i32,
    pub wins: i32,
    pub voters: usize,
    pub weight: f64,
    pub no_data: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct TrackRow {
    id: Uuid,
    spotify_id: String,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    preview_url: Option<String>,
    duration_ms: Option<i32>,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct VoteRow {
    user_id: Uuid,
    track_low: Uuid,
    track_high: Uuid,
    winner: Uuid,
}

#[derive(Debug, Default)]
struct Accumulator {
    weighted_sum: f64,
    weight: f64,
    voters: usize,
}

/// Read the singleton settings row, creating it with defaults if a very old
/// database somehow missed the migration seed.
pub async fn load_settings(pool: &PgPool) -> Result<BalancedSettings, ApiError> {
    let settings = sqlx::query_as::<_, BalancedSettings>(
        "SELECT influence_exponent, confidence_prior, neutral_prior, min_picks, updated_at \
         FROM balanced_settings WHERE id IS TRUE",
    )
    .fetch_optional(pool)
    .await?;

    if let Some(settings) = settings {
        return Ok(settings);
    }

    tracing::warn!("balanced settings row missing; seeding defaults");

    sqlx::query("INSERT INTO balanced_settings (id) VALUES (TRUE) ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;

    let settings = sqlx::query_as::<_, BalancedSettings>(
        "SELECT influence_exponent, confidence_prior, neutral_prior, min_picks, updated_at \
         FROM balanced_settings WHERE id IS TRUE",
    )
    .fetch_one(pool)
    .await?;

    Ok(settings)
}

/// Admin-only read of the current tuning constants.
pub async fn settings(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<BalancedSettings>, ApiError> {
    let settings = load_settings(&state.db).await?;

    tracing::debug!(
        influence_exponent = settings.influence_exponent,
        confidence_prior = settings.confidence_prior,
        neutral_prior = settings.neutral_prior,
        min_picks = settings.min_picks,
        "balanced settings loaded"
    );

    Ok(Json(settings))
}

/// Admin-only update of the tuning constants.
pub async fn update_settings(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<BalancedSettingsInput>,
) -> Result<Json<BalancedSettings>, ApiError> {
    validate_settings(&payload)?;
    let previous = load_settings(&state.db).await?;

    let updated = sqlx::query_as::<_, BalancedSettings>(
        "UPDATE balanced_settings \
         SET influence_exponent = $1, confidence_prior = $2, neutral_prior = $3, \
             min_picks = $4, updated_at = now() \
         WHERE id IS TRUE \
         RETURNING influence_exponent, confidence_prior, neutral_prior, min_picks, updated_at",
    )
    .bind(payload.influence_exponent)
    .bind(payload.confidence_prior)
    .bind(payload.neutral_prior)
    .bind(payload.min_picks)
    .fetch_one(&state.db)
    .await?;

    tracing::info!(
        admin_id = %admin.0.id,
        old_influence_exponent = previous.influence_exponent,
        old_confidence_prior = previous.confidence_prior,
        old_neutral_prior = previous.neutral_prior,
        old_min_picks = previous.min_picks,
        new_influence_exponent = updated.influence_exponent,
        new_confidence_prior = updated.confidence_prior,
        new_neutral_prior = updated.neutral_prior,
        new_min_picks = updated.min_picks,
        "balanced ranking settings updated"
    );

    Ok(Json(updated))
}

/// Public balanced list. Gated by The Reveal exactly like the official list.
pub async fn public(
    State(state): State<AppState>,
    Query(query): Query<BalancedQuery>,
) -> Result<Json<BalancedChart>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    if playlist.reveal_state != "revealed" {
        return Err(ApiError::Forbidden(
            "the standings have not been revealed yet".to_string(),
        ));
    }

    let settings = load_settings(&state.db).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    tracing::debug!(
        playlist_id = %playlist.id,
        limit,
        offset,
        query = %search,
        "public balanced standings requested"
    );

    Ok(Json(
        load_balanced_chart(
            &state.db,
            &playlist,
            &settings,
            limit,
            offset,
            &search,
        )
        .await?,
    ))
}

/// Admin view of the balanced list, available even while The Standings are sealed.
pub async fn admin_view(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<BalancedQuery>,
) -> Result<Json<BalancedChart>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let settings = load_settings(&state.db).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    tracing::debug!(
        playlist_id = %playlist.id,
        limit,
        offset,
        query = %search,
        "admin balanced standings requested"
    );

    Ok(Json(
        load_balanced_chart(
            &state.db,
            &playlist,
            &settings,
            limit,
            offset,
            &search,
        )
        .await?,
    ))
}

pub async fn load_balanced_chart(
    pool: &PgPool,
    playlist: &Playlist,
    settings: &BalancedSettings,
    limit: i64,
    offset: i64,
    search: &str,
) -> Result<BalancedChart, ApiError> {
    let started = Instant::now();
    let entries = compute_balanced(pool, playlist, settings).await?;
    let computed = entries.len();

    let needle = search.to_lowercase();
    let filtered: Vec<BalancedEntry> = entries
        .into_iter()
        .filter(|entry| {
            if needle.is_empty() {
                return true;
            }

            entry.track.title.to_lowercase().contains(&needle)
                || entry.track.artists.to_lowercase().contains(&needle)
                || entry
                    .track
                    .album
                    .as_deref()
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&needle)
        })
        .collect();

    let total = filtered.len() as i64;
    let page: Vec<BalancedEntry> = filtered
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(0))
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    tracing::info!(
        playlist_id = %playlist.id,
        computed_tracks = computed,
        matched_tracks = total,
        returned_tracks = page.len(),
        limit,
        offset,
        query = %search,
        influence_exponent = settings.influence_exponent,
        confidence_prior = settings.confidence_prior,
        neutral_prior = settings.neutral_prior,
        min_picks = settings.min_picks,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "balanced standings computed"
    );

    Ok(BalancedChart {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: playlist.reveal_state == "revealed",
        total,
        limit,
        offset,
        query: search.to_string(),
        tracks: page,
    })
}

/// Replay every voter's picks into a personal Elo, then combine the personal
/// deviations into one volume-adjusted list. Exposed for the CSV export.
pub async fn compute_balanced(
    pool: &PgPool,
    playlist: &Playlist,
    settings: &BalancedSettings,
) -> Result<Vec<BalancedEntry>, ApiError> {
    let tracks = sqlx::query_as::<_, TrackRow>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, comparisons, wins \
         FROM tracks WHERE playlist_id = $1 ORDER BY id",
    )
    .bind(playlist.id)
    .fetch_all(pool)
    .await?;

    let votes = sqlx::query_as::<_, VoteRow>(
        "SELECT user_id, track_low, track_high, winner FROM votes \
         WHERE playlist_id = $1 ORDER BY user_id ASC, created_at ASC, id ASC",
    )
    .bind(playlist.id)
    .fetch_all(pool)
    .await?;

    let by_id: HashMap<Uuid, TrackRow> =
        tracks.into_iter().map(|track| (track.id, track)).collect();

    let mut by_user: HashMap<Uuid, Vec<VoteRow>> = HashMap::new();
    for vote in votes {
        by_user.entry(vote.user_id).or_default().push(vote);
    }

    let min_picks = settings.min_picks.max(0) as usize;
    let mut accumulators: HashMap<Uuid, Accumulator> = HashMap::new();
    let mut voters_counted = 0usize;
    let mut voters_skipped = 0usize;

    for user_votes in by_user.values() {
        // A vote always references tracks in the playlist, but filter defensively
        // so a deleted track can never skew the replay.
        let valid: Vec<&VoteRow> = user_votes
            .iter()
            .filter(|vote| {
                by_id.contains_key(&vote.track_low) && by_id.contains_key(&vote.track_high)
            })
            .collect();
        let picks = valid.len();

        if picks == 0 || picks < min_picks {
            voters_skipped += 1;
            continue;
        }

        voters_counted += 1;
        let influence = influence_weight(picks, settings.influence_exponent);
        let mut ratings: HashMap<Uuid, f64> = HashMap::new();
        let mut comparisons: HashMap<Uuid, i32> = HashMap::new();

        for vote in valid {
            let low = vote.track_low;
            let high = vote.track_high;
            let low_rating = *ratings.entry(low).or_insert(elo::INITIAL_RATING);
            let high_rating = *ratings.entry(high).or_insert(elo::INITIAL_RATING);
            let low_comparisons = *comparisons.entry(low).or_insert(0);
            let high_comparisons = *comparisons.entry(high).or_insert(0);

            let outcome = elo::update(
                low_rating,
                high_rating,
                low_comparisons,
                high_comparisons,
                vote.winner == low,
            );

            *ratings.get_mut(&low).expect("known low track") += outcome.delta_low;
            *ratings.get_mut(&high).expect("known high track") += outcome.delta_high;
            *comparisons.get_mut(&low).expect("known low track") += 1;
            *comparisons.get_mut(&high).expect("known high track") += 1;
        }

        for (track_id, track_comparisons) in comparisons {
            let track_confidence = confidence(track_comparisons, settings.confidence_prior);
            if track_confidence <= 0.0 {
                continue;
            }

            let deviation = ratings
                .get(&track_id)
                .copied()
                .unwrap_or(elo::INITIAL_RATING)
                - elo::INITIAL_RATING;
            let effective = influence * track_confidence;

            let accumulator = accumulators.entry(track_id).or_default();
            accumulator.weighted_sum += effective * deviation;
            accumulator.weight += effective;
            accumulator.voters += 1;
        }
    }

    let mut entries: Vec<BalancedEntry> = by_id
        .into_values()
        .map(|track| {
            let accumulator = accumulators.remove(&track.id).unwrap_or_default();
            let score = if accumulator.weight > 0.0 {
                Some(combined_score(
                    accumulator.weighted_sum,
                    accumulator.weight,
                    settings.neutral_prior,
                ))
            } else {
                None
            };

            BalancedEntry {
                rank: None,
                track: MatchupTrack {
                    id: track.id,
                    spotify_id: track.spotify_id,
                    title: track.title,
                    artists: track.artists,
                    album: track.album,
                    album_image_url: track.album_image_url,
                    preview_url: track.preview_url,
                    duration_ms: track.duration_ms,
                },
                rating: score,
                comparisons: track.comparisons,
                wins: track.wins,
                voters: accumulator.voters,
                weight: accumulator.weight,
                no_data: score.is_none(),
            }
        })
        .collect();

    entries.sort_by(|left, right| match (left.rating, right.rating) {
        (Some(left_score), Some(right_score)) => right_score
            .partial_cmp(&left_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.wins.cmp(&left.wins))
            .then_with(|| left.track.title.cmp(&right.track.title))
            .then_with(|| left.track.id.cmp(&right.track.id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left
            .track
            .title
            .cmp(&right.track.title)
            .then_with(|| left.track.id.cmp(&right.track.id)),
    });

    let mut next_rank = 1usize;
    for entry in &mut entries {
        if entry.no_data {
            entry.rank = None;
        } else {
            entry.rank = Some(next_rank);
            next_rank += 1;
        }
    }

    tracing::debug!(
        playlist_id = %playlist.id,
        voters_total = by_user.len(),
        voters_counted,
        voters_skipped,
        tracks = entries.len(),
        influence_exponent = settings.influence_exponent,
        confidence_prior = settings.confidence_prior,
        neutral_prior = settings.neutral_prior,
        min_picks = settings.min_picks,
        "balanced replay finished"
    );

    Ok(entries)
}

fn validate_settings(input: &BalancedSettingsInput) -> Result<(), ApiError> {
    if !input.influence_exponent.is_finite()
        || !(0.0..=2.0).contains(&input.influence_exponent)
    {
        return Err(ApiError::bad_request(
            "influence exponent must be a number between 0 and 2",
        ));
    }

    if !input.confidence_prior.is_finite()
        || !(0.0..=500.0).contains(&input.confidence_prior)
    {
        return Err(ApiError::bad_request(
            "confidence prior must be a number between 0 and 500",
        ));
    }

    if !input.neutral_prior.is_finite() || !(0.0..=500.0).contains(&input.neutral_prior) {
        return Err(ApiError::bad_request(
            "neutral prior must be a number between 0 and 500",
        ));
    }

    if !(0..=10_000).contains(&input.min_picks) {
        return Err(ApiError::bad_request(
            "minimum picks must be an integer between 0 and 10000",
        ));
    }

    Ok(())
}

fn influence_weight(picks: usize, exponent: f64) -> f64 {
    if picks == 0 {
        return 0.0;
    }

    if exponent.abs() < f64::EPSILON {
        1.0
    } else {
        (picks as f64).powf(exponent)
    }
}

fn confidence(comparisons: i32, prior: f64) -> f64 {
    let comparisons = comparisons.max(0) as f64;

    if prior <= 0.0 {
        1.0
    } else {
        comparisons / (comparisons + prior)
    }
}

fn combined_score(weighted_sum: f64, weight: f64, neutral_prior: f64) -> f64 {
    if weight <= 0.0 {
        return elo::INITIAL_RATING;
    }

    elo::INITIAL_RATING + weighted_sum / (weight + neutral_prior.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-9
    }

    #[test]
    fn square_root_influence_is_the_default_curve() {
        assert!(close(influence_weight(20, 0.5), 20f64.sqrt()));
        assert!(close(influence_weight(100, 0.5), 10.0));
        assert!(close(influence_weight(1000, 0.5), 1000f64.sqrt()));
        assert!(
            influence_weight(1000, 0.5) < 10.0 * influence_weight(20, 0.5)
        );
    }

    #[test]
    fn zero_exponent_gives_every_voter_equal_influence() {
        assert!(close(influence_weight(1, 0.0), 1.0));
        assert!(close(influence_weight(20, 0.0), 1.0));
        assert!(close(influence_weight(1000, 0.0), 1.0));
        assert!(close(influence_weight(0, 0.0), 0.0));
    }

    #[test]
    fn confidence_shrinks_sparse_personal_scores() {
        assert!(close(confidence(0, 10.0), 0.0));
        assert!(close(confidence(10, 10.0), 0.5));
        assert!(close(confidence(30, 10.0), 0.75));
        assert!(close(confidence(10, 0.0), 1.0));
    }

    #[test]
    fn neutral_prior_pulls_songs_toward_one_thousand() {
        assert!(close(combined_score(200.0, 10.0, 10.0), 1010.0));
        assert!(close(combined_score(200.0, 10.0, 30.0), 1005.0));
        assert!(close(combined_score(0.0, 0.0, 20.0), 1000.0));
    }

    #[test]
    fn settings_validation_rejects_out_of_range_values() {
        let valid = BalancedSettingsInput {
            influence_exponent: 0.5,
            confidence_prior: 10.0,
            neutral_prior: 20.0,
            min_picks: 0,
        };
        assert!(validate_settings(&valid).is_ok());

        let bad_exponent = BalancedSettingsInput {
            influence_exponent: 3.0,
            ..valid
        };
        assert!(validate_settings(&bad_exponent).is_err());

        let bad_min = BalancedSettingsInput {
            influence_exponent: 0.5,
            confidence_prior: 10.0,
            neutral_prior: 20.0,
            min_picks: -1,
        };
        assert!(validate_settings(&bad_min).is_err());
    }
}
