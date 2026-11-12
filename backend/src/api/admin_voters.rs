//! Admin-only voter analytics: decision times, sessions, agreement, flags and
//! the crowd-level distributions.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::admin_stats::{cached, DuelPage, DuelsQuery, FlagCount};
use crate::api::analytics::{
    decision_bucket, decision_seconds, median, percentile, position_bucket, split_sessions,
    voter_flags, winner_side, Side, DECISION_BUCKETS, POSITION_BUCKETS,
};
use crate::api::settings::{stats_config, StatsConfig};
use crate::api::streak::resolve_timezone;
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::{validate_text_input, ApiError};
use crate::state::AppState;

const MAX_PAGE_SIZE: i64 = 100;

async fn playlist_id(state: &AppState) -> Result<Uuid, ApiError> {
    active_playlist(&state.db)
        .await?
        .map(|playlist| playlist.id)
        .ok_or_else(|| ApiError::NotFound("no active playlist has been imported yet".to_string()))
}

async fn heavy_permit(state: &AppState) -> Result<tokio::sync::SemaphorePermit<'_>, ApiError> {
    state
        .heavy_work
        .acquire()
        .await
        .map_err(|_| ApiError::service_unavailable("the server is busy; please try again"))
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct VoterBase {
    pub id: Uuid,
    pub username: String,
    pub is_admin: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserObservation {
    pub user_id: Uuid,
    pub seen_at: DateTime<Utc>,
    pub voted_at: Option<DateTime<Utc>>,
    pub winner: Option<Uuid>,
    pub served_flip: bool,
    pub track_low: Uuid,
    pub track_high: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoterStat {
    pub id: Uuid,
    pub username: String,
    pub is_admin: bool,
    pub disabled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub votes: i64,
    pub duels: i64,
    pub no_vote_rate: Option<f64>,
    pub median_decision_seconds: Option<f64>,
    pub agreement_pct: Option<f64>,
    pub sessions: i64,
    pub last_active_at: DateTime<Utc>,
    pub p10_decision_seconds: Option<f64>,
    pub p90_decision_seconds: Option<f64>,
    pub fast_share_pct: Option<f64>,
    pub side_bias_pct: Option<f64>,
    pub avg_duels_per_session: f64,
    pub total_voting_time_seconds: f64,
    pub distinct_songs_seen: i64,
    pub flags: crate::api::analytics::VoterFlags,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoterFlagSummary {
    pub key: String,
    pub label: String,
    pub voters: i64,
    pub vote_share_pct: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VotersPage {
    pub as_of: DateTime<Utc>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub query: String,
    pub sort: String,
    pub voters: Vec<VoterStat>,
    pub flag_summary: Vec<VoterFlagSummary>,
    pub flagged_voters: i64,
    pub flagged_vote_share_pct: f64,
}

#[derive(Debug, Default, Deserialize)]
pub struct VotersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
    pub sort: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct ObservationSql {
    user_id: Uuid,
    seen_at: DateTime<Utc>,
    voted_at: Option<DateTime<Utc>>,
    winner: Option<Uuid>,
    served_flip: bool,
    track_low: Uuid,
    track_high: Uuid,
}

impl From<ObservationSql> for UserObservation {
    fn from(row: ObservationSql) -> Self {
        Self {
            user_id: row.user_id,
            seen_at: row.seen_at,
            voted_at: row.voted_at,
            winner: row.winner,
            served_flip: row.served_flip,
            track_low: row.track_low,
            track_high: row.track_high,
        }
    }
}

async fn load_observations(
    pool: &PgPool,
    playlist_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<Vec<UserObservation>, ApiError> {
    let rows = if let Some(user_id) = user_id {
        sqlx::query_as::<_, ObservationSql>(
            "SELECT sp.user_id, sp.seen_at, v.created_at AS voted_at, v.winner, \
                    sp.served_flip, sp.track_low, sp.track_high \
             FROM seen_pairs sp \
             LEFT JOIN votes v \
               ON v.user_id = sp.user_id \
              AND v.playlist_id = sp.playlist_id \
              AND v.track_low = sp.track_low \
              AND v.track_high = sp.track_high \
             WHERE sp.playlist_id = $1 AND sp.user_id = $2 \
             ORDER BY sp.seen_at ASC",
        )
        .bind(playlist_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ObservationSql>(
            "SELECT sp.user_id, sp.seen_at, v.created_at AS voted_at, v.winner, \
                    sp.served_flip, sp.track_low, sp.track_high \
             FROM seen_pairs sp \
             LEFT JOIN votes v \
               ON v.user_id = sp.user_id \
              AND v.playlist_id = sp.playlist_id \
              AND v.track_low = sp.track_low \
              AND v.track_high = sp.track_high \
             WHERE sp.playlist_id = $1 \
             ORDER BY sp.user_id, sp.seen_at ASC",
        )
        .bind(playlist_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows.into_iter().map(UserObservation::from).collect())
}

async fn load_ratings(pool: &PgPool, playlist_id: Uuid) -> Result<HashMap<Uuid, f64>, ApiError> {
    let rows =
        sqlx::query_as::<_, (Uuid, f64)>("SELECT id, rating FROM tracks WHERE playlist_id = $1")
            .bind(playlist_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

async fn load_users(pool: &PgPool, search: &str) -> Result<Vec<VoterBase>, ApiError> {
    Ok(sqlx::query_as::<_, VoterBase>(
        "SELECT id, username, is_admin, disabled_at, created_at FROM users \
         WHERE ($1 = '' OR username ILIKE '%' || $1 || '%') \
         ORDER BY username ASC",
    )
    .bind(search)
    .fetch_all(pool)
    .await?)
}

fn agreement_for(observation: &UserObservation, ratings: &HashMap<Uuid, f64>) -> Option<bool> {
    let winner = observation.winner?;
    let other = if winner == observation.track_low {
        observation.track_high
    } else {
        observation.track_low
    };
    let picked = ratings.get(&winner)?;
    let opponent = ratings.get(&other)?;
    if (picked - opponent).abs() < 1e-9 {
        None
    } else {
        Some(picked > opponent)
    }
}

fn side_for(observation: &UserObservation) -> Option<Side> {
    let winner = observation.winner?;
    Some(winner_side(
        winner == observation.track_low,
        observation.served_flip,
    ))
}

/// Compute one row per account from the observations. Missing users have zero
/// duels and no flags.
pub fn compute_voter_stats(
    users: &[VoterBase],
    observations: &[UserObservation],
    ratings: &HashMap<Uuid, f64>,
    config: &StatsConfig,
) -> Vec<VoterStat> {
    let mut grouped: HashMap<Uuid, Vec<UserObservation>> = HashMap::new();
    for observation in observations {
        grouped
            .entry(observation.user_id)
            .or_default()
            .push(observation.clone());
    }
    for rows in grouped.values_mut() {
        rows.sort_by_key(|row| row.seen_at);
    }

    let gap_seconds = config.session_gap_seconds();
    users
        .iter()
        .map(|user| {
            let empty = Vec::new();
            let rows = grouped.get(&user.id).unwrap_or(&empty);
            let times = rows.iter().map(|row| row.seen_at).collect::<Vec<_>>();
            let sessions = split_sessions(&times, gap_seconds);

            let mut decisions = Vec::new();
            let mut votes = 0i64;
            let mut side_a = 0i64;
            let mut agreement_denominator = 0i64;
            let mut agreements = 0i64;
            let mut fast = 0i64;
            let mut songs = HashSet::new();
            let mut last_active = user.created_at;
            let mut total_voting_time = 0f64;

            for session in &sessions {
                let slice = &rows[session.start..session.end];
                if let (Some(first), Some(last)) = (slice.first(), slice.last()) {
                    total_voting_time += (last.seen_at - first.seen_at).num_seconds() as f64;
                }
                for observation in slice {
                    songs.insert(observation.track_low);
                    songs.insert(observation.track_high);
                    last_active = last_active.max(observation.seen_at);
                    let Some(voted_at) = observation.voted_at else {
                        continue;
                    };
                    votes += 1;
                    last_active = last_active.max(voted_at);
                    if side_for(observation) == Some(Side::A) {
                        side_a += 1;
                    }
                    if let Some(agrees) = agreement_for(observation, ratings) {
                        agreement_denominator += 1;
                        if agrees {
                            agreements += 1;
                        }
                    }
                    if let Some(seconds) =
                        decision_seconds(observation.seen_at, voted_at, gap_seconds)
                    {
                        if seconds < config.fast_decision_seconds {
                            fast += 1;
                        }
                        decisions.push(seconds);
                    }
                }
            }

            let duels = rows.len() as i64;
            let median_decision = median(&decisions);
            let side_bias = (votes > 0).then(|| side_a as f64 / votes as f64 * 100.0);
            let agreement = (agreement_denominator > 0)
                .then(|| agreements as f64 / agreement_denominator as f64 * 100.0);
            let fast_share =
                (votes > 0).then(|| fast as f64 / decisions.len().max(1) as f64 * 100.0);

            VoterStat {
                id: user.id,
                username: user.username.clone(),
                is_admin: user.is_admin,
                disabled_at: user.disabled_at,
                created_at: user.created_at,
                votes,
                duels,
                no_vote_rate: (duels > 0)
                    .then(|| (duels - votes).max(0) as f64 / duels as f64 * 100.0),
                median_decision_seconds: median_decision,
                agreement_pct: agreement,
                sessions: sessions.len() as i64,
                last_active_at: last_active,
                p10_decision_seconds: percentile(&decisions, 0.10),
                p90_decision_seconds: percentile(&decisions, 0.90),
                fast_share_pct: fast_share,
                side_bias_pct: side_bias,
                avg_duels_per_session: if sessions.is_empty() {
                    0.0
                } else {
                    duels as f64 / sessions.len() as f64
                },
                total_voting_time_seconds: total_voting_time,
                distinct_songs_seen: songs.len() as i64,
                flags: voter_flags(votes, median_decision, side_bias, agreement, config),
            }
        })
        .collect()
}

fn option_cmp(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    left.unwrap_or(f64::NEG_INFINITY)
        .total_cmp(&right.unwrap_or(f64::NEG_INFINITY))
}

pub fn sort_voter_stats(rows: &mut [VoterStat], sort: &str) {
    match sort {
        "username" => rows.sort_by(|a, b| a.username.cmp(&b.username)),
        "votes" => rows.sort_by(|a, b| b.votes.cmp(&a.votes).then(a.username.cmp(&b.username))),
        "duels" => rows.sort_by(|a, b| b.duels.cmp(&a.duels).then(a.username.cmp(&b.username))),
        "no_vote" => rows.sort_by(|a, b| {
            option_cmp(b.no_vote_rate, a.no_vote_rate).then(a.username.cmp(&b.username))
        }),
        "median_decision" => rows.sort_by(|a, b| {
            option_cmp(a.median_decision_seconds, b.median_decision_seconds)
                .then(a.username.cmp(&b.username))
        }),
        "agreement" => rows.sort_by(|a, b| {
            option_cmp(a.agreement_pct, b.agreement_pct).then(a.username.cmp(&b.username))
        }),
        "sessions" => rows.sort_by(|a, b| {
            b.sessions
                .cmp(&a.sessions)
                .then(a.username.cmp(&b.username))
        }),
        "p10" => rows.sort_by(|a, b| {
            option_cmp(a.p10_decision_seconds, b.p10_decision_seconds)
                .then(a.username.cmp(&b.username))
        }),
        "p90" => rows.sort_by(|a, b| {
            option_cmp(a.p90_decision_seconds, b.p90_decision_seconds)
                .then(a.username.cmp(&b.username))
        }),
        "fast_share" => rows.sort_by(|a, b| {
            option_cmp(b.fast_share_pct, a.fast_share_pct).then(a.username.cmp(&b.username))
        }),
        "side_bias" => rows.sort_by(|a, b| {
            option_cmp(b.side_bias_pct, a.side_bias_pct).then(a.username.cmp(&b.username))
        }),
        "avg_session" => rows.sort_by(|a, b| {
            b.avg_duels_per_session
                .total_cmp(&a.avg_duels_per_session)
                .then(a.username.cmp(&b.username))
        }),
        "voting_time" => rows.sort_by(|a, b| {
            b.total_voting_time_seconds
                .total_cmp(&a.total_voting_time_seconds)
                .then(a.username.cmp(&b.username))
        }),
        "songs" => rows.sort_by(|a, b| {
            b.distinct_songs_seen
                .cmp(&a.distinct_songs_seen)
                .then(a.username.cmp(&b.username))
        }),
        "flags" => rows.sort_by(|a, b| {
            b.flags
                .any()
                .cmp(&a.flags.any())
                .then(a.username.cmp(&b.username))
        }),
        _ => rows.sort_by(|a, b| {
            b.last_active_at
                .cmp(&a.last_active_at)
                .then(a.username.cmp(&b.username))
        }),
    }
}

fn summarize_flags(rows: &[VoterStat]) -> (Vec<VoterFlagSummary>, i64, f64) {
    let total_votes: i64 = rows.iter().map(|row| row.votes).sum();
    let mut summary = Vec::new();
    let mut flagged_voters = 0i64;
    let mut flagged_votes = 0i64;

    for (key, label, predicate) in [
        (
            "fast_clicker",
            "Fast clicker",
            (|row: &VoterStat| row.flags.fast_clicker) as fn(&VoterStat) -> bool,
        ),
        (
            "side_bias",
            "Side bias",
            (|row: &VoterStat| row.flags.side_bias) as fn(&VoterStat) -> bool,
        ),
        (
            "low_agreement",
            "Low agreement",
            (|row: &VoterStat| row.flags.low_agreement) as fn(&VoterStat) -> bool,
        ),
    ] {
        let selected = rows.iter().filter(|row| predicate(row)).collect::<Vec<_>>();
        let votes: i64 = selected.iter().map(|row| row.votes).sum();
        summary.push(VoterFlagSummary {
            key: key.to_string(),
            label: label.to_string(),
            voters: selected.len() as i64,
            vote_share_pct: if total_votes > 0 {
                votes as f64 / total_votes as f64 * 100.0
            } else {
                0.0
            },
        });
    }

    for row in rows {
        if row.flags.any() {
            flagged_voters += 1;
            flagged_votes += row.votes;
        }
    }

    let share = if total_votes > 0 {
        flagged_votes as f64 / total_votes as f64 * 100.0
    } else {
        0.0
    };
    (summary, flagged_voters, share)
}

/// Used by the vote-quality overview.
pub async fn flag_summary(
    state: &AppState,
    playlist_id: Uuid,
    config: &StatsConfig,
) -> Result<(i64, f64, HashMap<String, FlagCount>), ApiError> {
    let users = load_users(&state.db, "").await?;
    let observations = load_observations(&state.db, playlist_id, None).await?;
    let ratings = load_ratings(&state.db, playlist_id).await?;
    let stats = compute_voter_stats(&users, &observations, &ratings, config);
    let total_votes: i64 = stats.iter().map(|row| row.votes).sum();
    let mut flagged_voters = 0i64;
    let mut flagged_votes = 0i64;
    let mut counts = HashMap::new();

    for (key, predicate) in [
        (
            "fast_clicker",
            (|row: &VoterStat| row.flags.fast_clicker) as fn(&VoterStat) -> bool,
        ),
        (
            "side_bias",
            (|row: &VoterStat| row.flags.side_bias) as fn(&VoterStat) -> bool,
        ),
        (
            "low_agreement",
            (|row: &VoterStat| row.flags.low_agreement) as fn(&VoterStat) -> bool,
        ),
    ] {
        let selected = stats
            .iter()
            .filter(|row| predicate(row))
            .collect::<Vec<_>>();
        let votes: i64 = selected.iter().map(|row| row.votes).sum();
        counts.insert(
            key.to_string(),
            FlagCount {
                voters: selected.len() as i64,
                vote_share_pct: if total_votes > 0 {
                    votes as f64 / total_votes as f64 * 100.0
                } else {
                    0.0
                },
            },
        );
    }
    for row in &stats {
        if row.flags.any() {
            flagged_voters += 1;
            flagged_votes += row.votes;
        }
    }
    let share = if total_votes > 0 {
        flagged_votes as f64 / total_votes as f64 * 100.0
    } else {
        0.0
    };
    Ok((flagged_voters, share, counts))
}

/// All voter metrics for callers that need the whole room at once (the admin
/// Liner-notes stories). The paginated table has its own search path.
pub async fn all_voter_stats(
    pool: &PgPool,
    playlist_id: Uuid,
    config: &StatsConfig,
) -> Result<Vec<VoterStat>, ApiError> {
    let users = load_users(pool, "").await?;
    let observations = load_observations(pool, playlist_id, None).await?;
    let ratings = load_ratings(pool, playlist_id).await?;
    Ok(compute_voter_stats(&users, &observations, &ratings, config))
}

pub async fn voters(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<VotersQuery>,
) -> Result<Json<VotersPage>, ApiError> {
    let playlist_id = playlist_id(&state).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();
    validate_text_input(&search, "search")?;
    let sort = query.sort.unwrap_or_else(|| "last_active".to_string());

    let key = format!("admin:voters:{playlist_id}:{limit}:{offset}:{sort}:{search}");
    let page = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        voters_inner(&state, playlist_id, limit, offset, search, sort).await
    })
    .await?;
    Ok(Json(page))
}

async fn voters_inner(
    state: &AppState,
    playlist_id: Uuid,
    limit: i64,
    offset: i64,
    search: String,
    sort: String,
) -> Result<VotersPage, ApiError> {
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;

    let users = load_users(&state.db, &search).await?;
    let observations = load_observations(&state.db, playlist_id, None).await?;
    let ratings = load_ratings(&state.db, playlist_id).await?;
    let mut stats = compute_voter_stats(&users, &observations, &ratings, &config);
    let (flag_summary, flagged_voters, flagged_vote_share) = summarize_flags(&stats);

    sort_voter_stats(&mut stats, &sort);
    let total = stats.len() as i64;
    let page = stats
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect::<Vec<_>>();

    Ok(VotersPage {
        as_of: Utc::now(),
        total,
        limit,
        offset,
        query: search,
        sort,
        voters: page,
        flag_summary,
        flagged_voters,
        flagged_vote_share_pct: flagged_vote_share,
    })
}

// ---------------------------------------------------------------------------
// Voter detail
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SessionDecision {
    pub session: usize,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duels: i64,
    pub median_decision_seconds: Option<f64>,
    pub p10_decision_seconds: Option<f64>,
    pub p90_decision_seconds: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct SessionRow {
    pub session: usize,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub duration_seconds: f64,
    pub duels: i64,
    pub votes: i64,
    pub no_votes: i64,
    pub median_decision_seconds: Option<f64>,
    pub first10_decision_seconds: Option<f64>,
    pub last10_decision_seconds: Option<f64>,
    pub agreement_pct: Option<f64>,
    pub side_bias_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TirednessRow {
    pub label: String,
    pub decisions: i64,
    pub median_decision_seconds: Option<f64>,
    pub agreement_pct: Option<f64>,
    pub no_vote_rate: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct VoterDetail {
    pub as_of: DateTime<Utc>,
    pub voter: VoterStat,
    pub decision_histogram: Vec<HistogramBucket>,
    pub decisions_by_session: Vec<SessionDecision>,
    pub tiredness: Vec<TirednessRow>,
    pub sessions: Vec<SessionRow>,
    pub heatmap: crate::api::analytics::Heatmap,
    pub recent_duels: DuelPage,
}

#[derive(Debug, Default, Deserialize)]
pub struct VoterDetailQuery {
    pub tz: Option<String>,
    pub duel_limit: Option<i64>,
}

pub async fn voter_detail(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(user_id): Path<Uuid>,
    Query(query): Query<VoterDetailQuery>,
) -> Result<Json<VoterDetail>, ApiError> {
    let _permit = heavy_permit(&state).await?;
    let playlist_id = playlist_id(&state).await?;
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;
    let timezone = resolve_timezone(&mut connection, query.tz.as_deref()).await?;

    let users = load_users(&state.db, "").await?;
    let base = users
        .iter()
        .find(|user| user.id == user_id)
        .cloned()
        .ok_or_else(|| ApiError::NotFound("voter not found".to_string()))?;
    let observations = load_observations(&state.db, playlist_id, Some(user_id)).await?;
    let ratings = load_ratings(&state.db, playlist_id).await?;
    let stats = compute_voter_stats(
        std::slice::from_ref(&base),
        &observations,
        &ratings,
        &config,
    );
    let voter = stats
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound("voter not found".to_string()))?;

    let gap_seconds = config.session_gap_seconds();
    let times = observations
        .iter()
        .map(|row| row.seen_at)
        .collect::<Vec<_>>();
    let ranges = split_sessions(&times, gap_seconds);

    let mut decision_histogram_counts = [0i64; DECISION_BUCKETS.len()];
    let mut decisions_by_session = Vec::with_capacity(ranges.len());
    let mut sessions = Vec::with_capacity(ranges.len());
    let mut tired_decisions: Vec<Vec<f64>> =
        (0..POSITION_BUCKETS.len()).map(|_| Vec::new()).collect();
    let mut tired_agree = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_agree_den = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_no_vote = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_total = vec![0i64; POSITION_BUCKETS.len()];

    for (session_index, range) in ranges.iter().enumerate() {
        let slice = &observations[range.start..range.end];
        let mut decisions = Vec::new();
        let mut votes = 0i64;
        let mut side_a = 0i64;
        let mut agree = 0i64;
        let mut agree_den = 0i64;

        for (position, observation) in slice.iter().enumerate() {
            let bucket = position_bucket(position + 1);
            tired_total[bucket] += 1;
            if let Some(voted_at) = observation.voted_at {
                votes += 1;
                if side_for(observation) == Some(Side::A) {
                    side_a += 1;
                }
                if let Some(agrees) = agreement_for(observation, &ratings) {
                    agree_den += 1;
                    tired_agree_den[bucket] += 1;
                    if agrees {
                        agree += 1;
                        tired_agree[bucket] += 1;
                    }
                }
                if let Some(seconds) = decision_seconds(observation.seen_at, voted_at, gap_seconds)
                {
                    decisions.push(seconds);
                    tired_decisions[bucket].push(seconds);
                    decision_histogram_counts[decision_bucket(seconds)] += 1;
                }
            } else {
                tired_no_vote[bucket] += 1;
            }
        }

        let first10 = decisions.iter().copied().take(10).collect::<Vec<_>>();
        let last10 = decisions.iter().copied().rev().take(10).collect::<Vec<_>>();
        let started_at = slice
            .first()
            .map(|row| row.seen_at)
            .unwrap_or_else(Utc::now);
        let ended_at = slice.last().map(|row| row.seen_at).unwrap_or(started_at);
        let session_median = median(&decisions);
        decisions_by_session.push(SessionDecision {
            session: session_index + 1,
            started_at,
            ended_at,
            duels: slice.len() as i64,
            median_decision_seconds: session_median,
            p10_decision_seconds: percentile(&decisions, 0.10),
            p90_decision_seconds: percentile(&decisions, 0.90),
        });
        sessions.push(SessionRow {
            session: session_index + 1,
            started_at,
            ended_at,
            duration_seconds: (ended_at - started_at).num_seconds() as f64,
            duels: slice.len() as i64,
            votes,
            no_votes: slice.len() as i64 - votes,
            median_decision_seconds: session_median,
            first10_decision_seconds: median(&first10),
            last10_decision_seconds: median(&last10),
            agreement_pct: (agree_den > 0).then(|| agree as f64 / agree_den as f64 * 100.0),
            side_bias_pct: (votes > 0).then(|| side_a as f64 / votes as f64 * 100.0),
        });
    }

    let decision_histogram = DECISION_BUCKETS
        .iter()
        .zip(decision_histogram_counts)
        .map(|((label, _, _), count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    let tiredness = POSITION_BUCKETS
        .iter()
        .enumerate()
        .map(|(index, (label, _, _))| TirednessRow {
            label: (*label).to_string(),
            decisions: tired_decisions[index].len() as i64,
            median_decision_seconds: median(&tired_decisions[index]),
            agreement_pct: (tired_agree_den[index] > 0)
                .then(|| tired_agree[index] as f64 / tired_agree_den[index] as f64 * 100.0),
            no_vote_rate: (tired_total[index] > 0)
                .then(|| tired_no_vote[index] as f64 / tired_total[index] as f64 * 100.0),
        })
        .collect::<Vec<_>>();

    let heatmap_rows = sqlx::query_as::<_, (i32, i32, i64)>(
        "SELECT EXTRACT(DOW FROM seen_at AT TIME ZONE $3)::int AS dow, \
                EXTRACT(HOUR FROM seen_at AT TIME ZONE $3)::int AS hour, COUNT(*) \
         FROM seen_pairs WHERE user_id = $1 AND playlist_id = $2 \
         GROUP BY dow, hour",
    )
    .bind(user_id)
    .bind(playlist_id)
    .bind(&timezone)
    .fetch_all(&state.db)
    .await?;
    let mut cells = vec![vec![0i64; 24]; 7];
    for (dow, hour, count) in heatmap_rows {
        cells[dow.clamp(0, 6) as usize][hour.clamp(0, 23) as usize] = count;
    }

    let recent_duels = crate::api::admin_stats::duels_page_for(
        &state,
        playlist_id,
        &config,
        DuelsQuery {
            user_id: Some(user_id),
            limit: Some(query.duel_limit.unwrap_or(10).clamp(1, 50)),
            ..DuelsQuery::default()
        },
    )
    .await?;

    Ok(Json(VoterDetail {
        as_of: Utc::now(),
        voter,
        decision_histogram,
        decisions_by_session,
        tiredness,
        sessions,
        heatmap: crate::api::analytics::Heatmap::new(cells),
        recent_duels,
    }))
}

// ---------------------------------------------------------------------------
// Crowd level
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct CrowdTirednessRow {
    pub label: String,
    pub decisions: i64,
    pub median_decision_seconds: Option<f64>,
    pub agreement_pct: Option<f64>,
    pub no_vote_rate: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrowdResponse {
    pub as_of: DateTime<Utc>,
    pub voters: i64,
    pub decision_time_distribution: Vec<HistogramBucket>,
    pub agreement_distribution: Vec<HistogramBucket>,
    pub session_length_distribution: Vec<HistogramBucket>,
    pub tiredness: Vec<CrowdTirednessRow>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CrowdQuery {
    pub _unused: Option<String>,
}

pub async fn crowd(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(_query): Query<CrowdQuery>,
) -> Result<Json<CrowdResponse>, ApiError> {
    let playlist_id = playlist_id(&state).await?;
    let key = format!("admin:crowd:{playlist_id}");
    let value = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        crowd_inner(&state, playlist_id).await
    })
    .await?;
    Ok(Json(value))
}

async fn crowd_inner(state: &AppState, playlist_id: Uuid) -> Result<CrowdResponse, ApiError> {
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;

    let users = load_users(&state.db, "").await?;
    let observations = load_observations(&state.db, playlist_id, None).await?;
    let ratings = load_ratings(&state.db, playlist_id).await?;
    let stats = compute_voter_stats(&users, &observations, &ratings, &config);

    let mut decision_counts = [0i64; DECISION_BUCKETS.len()];
    for row in &stats {
        if let Some(seconds) = row.median_decision_seconds {
            decision_counts[decision_bucket(seconds)] += 1;
        }
    }
    let decision_time_distribution = DECISION_BUCKETS
        .iter()
        .zip(decision_counts)
        .map(|((label, _, _), count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    let agreement_labels = ["<50%", "50–54%", "55–59%", "60–69%", "70–79%", "80%+"];
    let mut agreement_counts = [0i64; 6];
    for row in &stats {
        if let Some(value) = row.agreement_pct {
            let index = if value < 50.0 {
                0
            } else if value < 55.0 {
                1
            } else if value < 60.0 {
                2
            } else if value < 70.0 {
                3
            } else if value < 80.0 {
                4
            } else {
                5
            };
            agreement_counts[index] += 1;
        }
    }
    let agreement_distribution = agreement_labels
        .iter()
        .zip(agreement_counts)
        .map(|(label, count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    let mut grouped: HashMap<Uuid, Vec<&UserObservation>> = HashMap::new();
    for observation in &observations {
        grouped
            .entry(observation.user_id)
            .or_default()
            .push(observation);
    }
    for rows in grouped.values_mut() {
        rows.sort_by_key(|row| row.seen_at);
    }

    let session_labels = ["1–5", "6–10", "11–25", "26–50", "51–100", "100+"];
    let mut session_counts = [0i64; 6];
    let mut tired_decisions: Vec<Vec<f64>> =
        (0..POSITION_BUCKETS.len()).map(|_| Vec::new()).collect();
    let mut tired_agree = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_agree_den = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_no_vote = vec![0i64; POSITION_BUCKETS.len()];
    let mut tired_total = vec![0i64; POSITION_BUCKETS.len()];
    let gap_seconds = config.session_gap_seconds();

    for rows in grouped.values() {
        let times = rows.iter().map(|row| row.seen_at).collect::<Vec<_>>();
        for range in split_sessions(&times, gap_seconds) {
            let length = range.len() as i64;
            let index = match length {
                0..=5 => 0,
                6..=10 => 1,
                11..=25 => 2,
                26..=50 => 3,
                51..=100 => 4,
                _ => 5,
            };
            session_counts[index] += 1;

            for (position, observation) in rows[range.start..range.end].iter().enumerate() {
                let bucket = position_bucket(position + 1);
                tired_total[bucket] += 1;
                if let Some(voted_at) = observation.voted_at {
                    if let Some(agrees) = agreement_for(observation, &ratings) {
                        tired_agree_den[bucket] += 1;
                        if agrees {
                            tired_agree[bucket] += 1;
                        }
                    }
                    if let Some(seconds) =
                        decision_seconds(observation.seen_at, voted_at, gap_seconds)
                    {
                        tired_decisions[bucket].push(seconds);
                    }
                } else {
                    tired_no_vote[bucket] += 1;
                }
            }
        }
    }

    let session_length_distribution = session_labels
        .iter()
        .zip(session_counts)
        .map(|(label, count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    let tiredness = POSITION_BUCKETS
        .iter()
        .enumerate()
        .map(|(index, (label, _, _))| CrowdTirednessRow {
            label: (*label).to_string(),
            decisions: tired_decisions[index].len() as i64,
            median_decision_seconds: median(&tired_decisions[index]),
            agreement_pct: (tired_agree_den[index] > 0)
                .then(|| tired_agree[index] as f64 / tired_agree_den[index] as f64 * 100.0),
            no_vote_rate: (tired_total[index] > 0)
                .then(|| tired_no_vote[index] as f64 / tired_total[index] as f64 * 100.0),
        })
        .collect::<Vec<_>>();

    Ok(CrowdResponse {
        as_of: Utc::now(),
        voters: stats.len() as i64,
        decision_time_distribution,
        agreement_distribution,
        session_length_distribution,
        tiredness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::analytics::VoterFlags;
    use chrono::{TimeZone, Utc};

    fn at(second: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(second, 0).unwrap()
    }

    fn config() -> StatsConfig {
        StatsConfig {
            session_gap_minutes: 15,
            fast_decision_seconds: 1.5,
            upset_threshold_pct: 35.0,
            side_bias_margin_pct: 15.0,
            low_agreement_pct: 55.0,
            min_votes_for_flags: 3,
            target_duels_per_song: 30.0,
        }
    }

    fn user(id: u128, name: &str) -> VoterBase {
        VoterBase {
            id: Uuid::from_u128(id),
            username: name.to_string(),
            is_admin: false,
            disabled_at: None,
            created_at: at(0),
        }
    }

    fn obs(
        user_id: u128,
        seen: i64,
        voted: Option<i64>,
        winner: Option<u128>,
        flip: bool,
    ) -> UserObservation {
        UserObservation {
            user_id: Uuid::from_u128(user_id),
            seen_at: at(seen),
            voted_at: voted.map(at),
            winner: winner.map(Uuid::from_u128),
            served_flip: flip,
            track_low: Uuid::from_u128(1),
            track_high: Uuid::from_u128(2),
        }
    }

    #[test]
    fn agreement_requires_a_winner_and_both_ratings() {
        let ratings = HashMap::from([(Uuid::from_u128(1), 1100.0), (Uuid::from_u128(2), 900.0)]);
        assert_eq!(agreement_for(&obs(1, 0, None, None, false), &ratings), None);
        assert_eq!(
            agreement_for(&obs(1, 0, Some(1), Some(99), false), &ratings),
            None
        );
        assert_eq!(
            agreement_for(&obs(1, 0, Some(1), Some(1), false), &ratings),
            Some(true)
        );
        assert_eq!(
            agreement_for(&obs(1, 0, Some(1), Some(2), false), &ratings),
            Some(false)
        );
    }

    #[test]
    fn agreement_ignores_equal_ratings() {
        let ratings = HashMap::from([(Uuid::from_u128(1), 1000.0), (Uuid::from_u128(2), 1000.0)]);
        assert_eq!(
            agreement_for(&obs(1, 0, Some(1), Some(1), false), &ratings),
            None
        );
    }

    #[test]
    fn side_for_maps_the_served_orientation() {
        assert_eq!(side_for(&obs(1, 0, Some(1), Some(1), false)), Some(Side::A));
        assert_eq!(side_for(&obs(1, 0, Some(1), Some(1), true)), Some(Side::B));
        assert_eq!(side_for(&obs(1, 0, Some(1), Some(2), false)), Some(Side::B));
        assert_eq!(side_for(&obs(1, 0, Some(1), Some(2), true)), Some(Side::A));
        assert_eq!(side_for(&obs(1, 0, None, None, false)), None);
    }

    #[test]
    fn compute_stats_keeps_users_without_observations() {
        let stats = compute_voter_stats(&[user(1, "quiet")], &[], &HashMap::new(), &config());
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].votes, 0);
        assert_eq!(stats[0].duels, 0);
        assert_eq!(stats[0].sessions, 0);
        assert_eq!(stats[0].avg_duels_per_session, 0.0);
        assert!(!stats[0].flags.any());
    }

    #[test]
    fn compute_stats_counts_votes_and_skips() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 10, None, None, false),
            obs(1, 20, Some(1), Some(2), false),
        ];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].duels, 3);
        assert_eq!(stats[0].votes, 2);
        assert!((stats[0].no_vote_rate.unwrap() - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn compute_stats_groups_observations_by_user() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(2, 0, Some(1), Some(1), false),
            obs(1, 10, Some(1), Some(1), false),
        ];
        let users = [user(1, "one"), user(2, "two")];
        let stats = compute_voter_stats(&users, &observations, &HashMap::new(), &config());
        assert_eq!(stats[0].votes, 2);
        assert_eq!(stats[1].votes, 1);
    }

    #[test]
    fn compute_stats_splits_sessions_on_the_configured_gap() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 60, Some(1), Some(1), false),
            obs(1, 60 + 15 * 60, Some(1), Some(1), false),
            obs(1, 60 + 15 * 60 + 901, Some(1), Some(1), false),
        ];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].sessions, 2);
        assert_eq!(stats[0].avg_duels_per_session, 2.0);
    }

    #[test]
    fn compute_stats_ignores_idle_and_negative_decision_times() {
        let mut idle = obs(1, 0, Some(1000), Some(1), false);
        idle.seen_at = at(0);
        let observations = vec![idle, obs(1, 2000, Some(2001), Some(1), false)];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].median_decision_seconds, Some(1.0));
    }

    #[test]
    fn compute_stats_calculates_side_bias() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 10, Some(1), Some(1), false),
            obs(1, 20, Some(1), Some(2), false),
        ];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert!((stats[0].side_bias_pct.unwrap() - 200.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn compute_stats_calculates_agreement_percentage() {
        let ratings = HashMap::from([(Uuid::from_u128(1), 1100.0), (Uuid::from_u128(2), 900.0)]);
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 10, Some(1), Some(2), false),
        ];
        let stats = compute_voter_stats(&[user(1, "voter")], &observations, &ratings, &config());
        assert_eq!(stats[0].agreement_pct, Some(50.0));
    }

    #[test]
    fn compute_stats_counts_distinct_songs_seen() {
        let mut first = obs(1, 0, Some(1), Some(1), false);
        first.track_low = Uuid::from_u128(1);
        first.track_high = Uuid::from_u128(2);
        let mut second = obs(1, 10, Some(1), Some(1), false);
        second.track_low = Uuid::from_u128(2);
        second.track_high = Uuid::from_u128(3);
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &[first, second],
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].distinct_songs_seen, 3);
    }

    #[test]
    fn compute_stats_excludes_non_votes_from_decision_percentiles() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 10, None, None, false),
        ];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].median_decision_seconds, Some(1.0));
        assert_eq!(stats[0].p10_decision_seconds, Some(1.0));
    }

    #[test]
    fn compute_stats_marks_fast_clickers() {
        let observations = vec![
            obs(1, 0, Some(1), Some(1), false),
            obs(1, 10, Some(11), Some(1), false),
            obs(1, 20, Some(21), Some(1), false),
        ];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert!(stats[0].flags.fast_clicker);
    }

    #[test]
    fn compute_stats_marks_low_agreement_only_after_the_minimum_votes() {
        let ratings = HashMap::from([(Uuid::from_u128(1), 1100.0), (Uuid::from_u128(2), 900.0)]);
        let observations = vec![
            obs(1, 0, Some(1), Some(2), false),
            obs(1, 10, Some(1), Some(2), false),
            obs(1, 20, Some(1), Some(2), false),
        ];
        let stats = compute_voter_stats(&[user(1, "voter")], &observations, &ratings, &config());
        assert!(stats[0].flags.low_agreement);
    }

    #[test]
    fn compute_stats_marks_side_bias_beyond_the_margin() {
        let observations = (0..10)
            .map(|index| obs(1, index * 10, Some(index * 10 + 10), Some(1), false))
            .collect::<Vec<_>>();
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert!(stats[0].flags.side_bias);
    }

    #[test]
    fn compute_stats_preserves_account_metadata() {
        let mut voter = user(1, "voter");
        voter.is_admin = true;
        voter.disabled_at = Some(at(100));
        let stats = compute_voter_stats(&[voter], &[], &HashMap::new(), &config());
        assert!(stats[0].is_admin);
        assert_eq!(stats[0].disabled_at, Some(at(100)));
    }

    #[test]
    fn compute_stats_uses_vote_time_for_last_active() {
        let observations = vec![obs(1, 100, Some(500), Some(1), false)];
        let stats = compute_voter_stats(
            &[user(1, "voter")],
            &observations,
            &HashMap::new(),
            &config(),
        );
        assert_eq!(stats[0].last_active_at, at(500));
    }

    fn stat(name: &str, votes: i64, duels: i64) -> VoterStat {
        VoterStat {
            id: Uuid::from_u128(votes as u128 + 1),
            username: name.to_string(),
            is_admin: false,
            disabled_at: None,
            created_at: at(0),
            votes,
            duels,
            no_vote_rate: if duels > 0 {
                Some((duels - votes) as f64 / duels as f64 * 100.0)
            } else {
                None
            },
            median_decision_seconds: Some(votes as f64),
            agreement_pct: Some(votes as f64 * 2.0),
            sessions: votes,
            last_active_at: at(votes),
            p10_decision_seconds: Some(votes as f64 / 2.0),
            p90_decision_seconds: Some(votes as f64 * 1.5),
            fast_share_pct: Some(votes as f64 * 3.0),
            side_bias_pct: Some(votes as f64 * 4.0),
            avg_duels_per_session: votes as f64,
            total_voting_time_seconds: votes as f64 * 10.0,
            distinct_songs_seen: votes * 2,
            flags: VoterFlags {
                fast_clicker: votes % 2 == 0,
                side_bias: votes % 3 == 0,
                low_agreement: votes % 5 == 0,
            },
        }
    }

    #[test]
    fn sort_by_username_is_alphabetical() {
        let mut rows = vec![stat("b", 1, 1), stat("a", 2, 2)];
        sort_voter_stats(&mut rows, "username");
        assert_eq!(rows[0].username, "a");
        assert_eq!(rows[1].username, "b");
    }

    #[test]
    fn sort_by_votes_is_descending_with_username_tiebreak() {
        let mut rows = vec![stat("b", 3, 3), stat("a", 3, 3), stat("c", 1, 1)];
        sort_voter_stats(&mut rows, "votes");
        assert_eq!(
            rows.iter()
                .map(|row| row.username.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
    }

    #[test]
    fn sort_by_duels_is_descending() {
        let mut rows = vec![stat("a", 1, 10), stat("b", 2, 20)];
        sort_voter_stats(&mut rows, "duels");
        assert_eq!(rows[0].username, "b");
    }

    #[test]
    fn sort_by_no_vote_includes_missing_values_last() {
        let mut rows = vec![stat("a", 1, 1), stat("b", 1, 0)];
        sort_voter_stats(&mut rows, "no_vote");
        assert_eq!(rows[0].username, "a");
        assert_eq!(rows[1].username, "b");
    }

    #[test]
    fn sort_by_median_decision_puts_missing_values_last() {
        let mut rows = vec![stat("a", 2, 2), stat("b", 1, 0)];
        rows[1].median_decision_seconds = None;
        sort_voter_stats(&mut rows, "median_decision");
        assert_eq!(rows[0].username, "b");
        assert_eq!(rows[1].username, "a");
    }

    #[test]
    fn sort_by_agreement_is_ascending_so_low_agreement_surfaces_first() {
        let mut rows = vec![stat("a", 1, 1), stat("b", 2, 2)];
        sort_voter_stats(&mut rows, "agreement");
        assert_eq!(rows[0].username, "a");
    }

    #[test]
    fn sort_by_sessions_is_descending() {
        let mut rows = vec![stat("a", 1, 1), stat("b", 3, 3)];
        sort_voter_stats(&mut rows, "sessions");
        assert_eq!(rows[0].username, "b");
    }

    #[test]
    fn sort_by_percentile_columns_is_ascending() {
        for key in ["p10", "p90"] {
            let mut rows = vec![stat("a", 3, 3), stat("b", 1, 1)];
            sort_voter_stats(&mut rows, key);
            assert_eq!(rows[0].username, "b", "sort {key}");
        }
    }

    #[test]
    fn sort_by_share_columns_is_descending() {
        for key in [
            "fast_share",
            "side_bias",
            "avg_session",
            "voting_time",
            "songs",
        ] {
            let mut rows = vec![stat("a", 1, 1), stat("b", 2, 2)];
            sort_voter_stats(&mut rows, key);
            assert_eq!(rows[0].username, "b", "sort {key}");
        }
    }

    #[test]
    fn sort_by_flags_puts_flagged_voters_first() {
        let mut rows = vec![stat("a", 1, 1), stat("b", 2, 2)];
        sort_voter_stats(&mut rows, "flags");
        assert_eq!(rows[0].username, "b");
    }

    #[test]
    fn unknown_sort_defaults_to_last_active() {
        let mut rows = vec![stat("a", 1, 1), stat("b", 3, 3)];
        sort_voter_stats(&mut rows, "not-a-column");
        assert_eq!(rows[0].username, "b");
    }

    #[test]
    fn flag_summary_counts_voters_and_vote_share() {
        let rows = vec![stat("a", 2, 2), stat("b", 3, 3), stat("c", 5, 5)];
        let (summary, flagged_voters, share) = summarize_flags(&rows);
        assert_eq!(summary.len(), 3);
        assert_eq!(summary[0].key, "fast_clicker");
        assert_eq!(summary[0].voters, 1);
        assert_eq!(summary[0].vote_share_pct, 20.0);
        assert_eq!(flagged_voters, 3);
        assert_eq!(share, 100.0);
    }

    #[test]
    fn flag_summary_handles_no_votes() {
        let rows = vec![];
        let (summary, flagged_voters, share) = summarize_flags(&rows);
        assert_eq!(summary.len(), 3);
        assert!(summary
            .iter()
            .all(|row| row.voters == 0 && row.vote_share_pct == 0.0));
        assert_eq!(flagged_voters, 0);
        assert_eq!(share, 0.0);
    }
}
