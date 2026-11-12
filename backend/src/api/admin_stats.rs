//! Deep, admin-only statistics for mission control.
//!
//! Everything here is read-only and lives behind `/api/admin/...`. Heavy
//! handlers take the shared `heavy_work` permit so live voting cannot be
//! starved by analytics.

use std::collections::HashMap;
use std::future::Future;

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::analytics::{
    self, duels_left, evenness, is_upset, median, percentile, rank_bucket, rank_bucket_index,
    rank_change, rank_map_at, rating_after, settling_status, win_chance, winner_side, Heatmap,
    RatingTimeline, SettlingStatus, RANK_BUCKETS,
};
use crate::api::settings::{rank_weighting_config, stats_config, StatsConfig};
use crate::api::streak::resolve_timezone;
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::{validate_text_input, ApiError};
use crate::state::AppState;

const INITIAL_RATING: f64 = 1000.0;
const MAX_PAGE_SIZE: i64 = 100;
const DEFAULT_PAGE_SIZE: i64 = 15;

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

/// Cache a typed aggregate as JSON for a short while. Every response carries
/// its own `as_of`, so the UI can say how fresh the numbers are.
pub(crate) async fn cached<T, F, Fut>(
    state: &AppState,
    key: String,
    ttl_secs: u64,
    compute: F,
) -> Result<T, ApiError>
where
    T: Serialize + serde::de::DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, ApiError>>,
{
    let value = state
        .cache
        .get_or_compute(key, std::time::Duration::from_secs(ttl_secs), || async {
            let value = compute().await?;
            serde_json::to_value(value).map_err(ApiError::internal)
        })
        .await?;
    serde_json::from_value(value).map_err(ApiError::internal)
}

fn parse_time(value: Option<&str>, label: &str) -> Result<Option<DateTime<Utc>>, ApiError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    validate_text_input(value, label)?;
    DateTime::parse_from_rfc3339(value)
        .map(|value| Some(value.with_timezone(&Utc)))
        .map_err(|_| ApiError::bad_request(format!("{label} must be an RFC 3339 timestamp")))
}

fn window_start(
    hours: Option<f64>,
    from: Option<&str>,
    default_hours: f64,
) -> Result<DateTime<Utc>, ApiError> {
    if let Some(from) = parse_time(from, "from")? {
        return Ok(from);
    }
    let hours = clamped_hours(hours, default_hours, 24.0 * 120.0);
    Ok(Utc::now() - Duration::seconds((hours * 3600.0).round() as i64))
}

/// Clamp a user-supplied lookback window. Non-finite values (for example
/// `hours=NaN` from a hand-written query) must not reach `chrono`, where a NaN
/// cast would silently collapse the window to zero.
fn clamped_hours(value: Option<f64>, default: f64, max: f64) -> f64 {
    value
        .filter(|hours| hours.is_finite())
        .unwrap_or(default)
        .clamp(0.01, max)
}

// ---------------------------------------------------------------------------
// Duel feed
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct DuelsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub only: Option<String>,
    /// `recent` (default) or `upsets`.
    pub sort: Option<String>,
    pub user_id: Option<Uuid>,
    pub track_id: Option<Uuid>,
    pub q: Option<String>,
    /// Username substring filter.
    pub user: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    /// Convenience window; ignored when `from` is set.
    pub hours: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuelPage {
    pub as_of: DateTime<Utc>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub only: String,
    pub sort: String,
    pub duels: Vec<DuelRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelTrack {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub release_year: Option<i32>,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub current_rank: i64,
    pub current_rating: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelDetails {
    pub served_flip: bool,
    pub rating_a_before: Option<f64>,
    pub rating_b_before: Option<f64>,
    pub rating_a_after: Option<f64>,
    pub rating_b_after: Option<f64>,
    pub win_chance_a: Option<f64>,
    pub win_chance_b: Option<f64>,
    pub rating_gap_before: Option<f64>,
    pub delta_a: Option<f64>,
    pub delta_b: Option<f64>,
    pub comparisons_a_before: Option<i32>,
    pub comparisons_b_before: Option<i32>,
    pub k_factor: Option<f64>,
    pub vote_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuelRow {
    pub seen_at: DateTime<Utc>,
    pub voted_at: Option<DateTime<Utc>>,
    pub user_id: Uuid,
    pub username: String,
    pub track_a: DuelTrack,
    pub track_b: DuelTrack,
    pub result_side: Option<String>,
    pub winner_id: Option<Uuid>,
    pub no_vote: bool,
    pub upset: bool,
    pub win_chance_pct: Option<f64>,
    pub rating_swing: Option<f64>,
    pub decision_seconds: Option<f64>,
    pub idle: bool,
    pub details: DuelDetails,
}

#[derive(Debug, sqlx::FromRow)]
struct DuelSqlRow {
    user_id: Uuid,
    username: String,
    seen_at: DateTime<Utc>,
    served_flip: bool,
    track_low: Uuid,
    track_high: Uuid,
    winner: Option<Uuid>,
    voted_at: Option<DateTime<Utc>>,
    rating_low_before: Option<f64>,
    rating_high_before: Option<f64>,
    delta_low: Option<f64>,
    delta_high: Option<f64>,
    comparisons_low_before: Option<i32>,
    comparisons_high_before: Option<i32>,
    k_factor: Option<f64>,
    seq: Option<i64>,
    low_title: String,
    low_artists: String,
    low_release_year: Option<i32>,
    low_image_url: Option<String>,
    low_preview_url: Option<String>,
    high_title: String,
    high_artists: String,
    high_release_year: Option<i32>,
    high_image_url: Option<String>,
    high_preview_url: Option<String>,
    low_rank: i64,
    high_rank: i64,
    low_current_rating: f64,
    high_current_rating: f64,
    winner_win_chance: Option<f64>,
    decision_seconds: Option<f64>,
}

const DUEL_SELECT_CTE: &str = r#"
WITH selected AS (
    SELECT sp.user_id, u.username, sp.seen_at, sp.served_flip,
           sp.track_low, sp.track_high, v.winner, v.created_at AS voted_at,
           v.rating_low_before, v.rating_high_before, v.delta_low, v.delta_high,
           v.comparisons_low_before, v.comparisons_high_before, v.k_factor, v.seq,
           EXTRACT(EPOCH FROM (v.created_at - sp.seen_at))::float8 AS decision_seconds,
           CASE WHEN v.winner IS NULL THEN NULL
                WHEN v.winner = sp.track_low
                  THEN 100.0 / (1.0 + power(10.0::float8,
                       (v.rating_high_before - v.rating_low_before) / 400.0))
                ELSE 100.0 / (1.0 + power(10.0::float8,
                       (v.rating_low_before - v.rating_high_before) / 400.0))
           END AS winner_win_chance
    FROM seen_pairs sp
    JOIN users u ON u.id = sp.user_id
    LEFT JOIN votes v
      ON v.user_id = sp.user_id
     AND v.playlist_id = sp.playlist_id
     AND v.track_low = sp.track_low
     AND v.track_high = sp.track_high
    WHERE sp.playlist_id = $1
      AND ($2::uuid IS NULL OR sp.user_id = $2)
      AND ($3::uuid IS NULL OR sp.track_low = $3 OR sp.track_high = $3)
      AND ($4::uuid[] IS NULL
           OR sp.track_low = ANY($4) OR sp.track_high = ANY($4))
      AND ($5::timestamptz IS NULL OR sp.seen_at >= $5)
      AND ($6::timestamptz IS NULL OR sp.seen_at < $6)
      AND ($7::text = 'all'
           OR ($7::text = 'voted' AND v.winner IS NOT NULL)
           OR ($7::text = 'no_vote' AND v.winner IS NULL)
           OR ($7::text = 'upset' AND v.winner IS NOT NULL
               AND CASE WHEN v.winner = sp.track_low
                     THEN 100.0 / (1.0 + power(10.0::float8,
                          (v.rating_high_before - v.rating_low_before) / 400.0))
                     ELSE 100.0 / (1.0 + power(10.0::float8,
                          (v.rating_low_before - v.rating_high_before) / 400.0))
                END < $8))
      AND ($9::text = '' OR u.username ILIKE '%' || $9 || '%')
),
page AS (
    SELECT * FROM selected
    ORDER BY {inner_order}
    LIMIT $10 OFFSET $11
),
ranked AS (
    SELECT id, rating,
           ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) AS current_rank
    FROM tracks
    WHERE playlist_id = $1
)
SELECT p.*,
       tl.title AS low_title, tl.artists AS low_artists,
       tl.release_year AS low_release_year, tl.album_image_url AS low_image_url,
       tl.preview_url AS low_preview_url,
       th.title AS high_title, th.artists AS high_artists,
       th.release_year AS high_release_year, th.album_image_url AS high_image_url,
       th.preview_url AS high_preview_url,
       rl.current_rank AS low_rank, rh.current_rank AS high_rank,
       rl.rating AS low_current_rating, rh.rating AS high_current_rating
FROM page p
JOIN tracks tl ON tl.id = p.track_low
JOIN tracks th ON th.id = p.track_high
JOIN ranked rl ON rl.id = p.track_low
JOIN ranked rh ON rh.id = p.track_high
ORDER BY {outer_order}
"#;

const DUEL_COUNT_CTE: &str = r#"
WITH rows AS (
    SELECT sp.user_id, sp.seen_at, sp.track_low, sp.track_high, u.username, v.winner,
           CASE WHEN v.winner IS NULL THEN NULL
                WHEN v.winner = sp.track_low
                  THEN 100.0 / (1.0 + power(10.0::float8,
                       (v.rating_high_before - v.rating_low_before) / 400.0))
                ELSE 100.0 / (1.0 + power(10.0::float8,
                       (v.rating_low_before - v.rating_high_before) / 400.0))
           END AS winner_win_chance
    FROM seen_pairs sp
    JOIN users u ON u.id = sp.user_id
    LEFT JOIN votes v
      ON v.user_id = sp.user_id
     AND v.playlist_id = sp.playlist_id
     AND v.track_low = sp.track_low
     AND v.track_high = sp.track_high
    WHERE sp.playlist_id = $1
      AND ($2::uuid IS NULL OR sp.user_id = $2)
      AND ($3::uuid IS NULL OR sp.track_low = $3 OR sp.track_high = $3)
      AND ($4::uuid[] IS NULL
           OR sp.track_low = ANY($4) OR sp.track_high = ANY($4))
      AND ($5::timestamptz IS NULL OR sp.seen_at >= $5)
      AND ($6::timestamptz IS NULL OR sp.seen_at < $6)
      AND ($9::text = '' OR u.username ILIKE '%' || $9 || '%')
)
SELECT COUNT(*) FROM rows
WHERE ($7::text = 'all'
       OR ($7::text = 'voted' AND winner IS NOT NULL)
       OR ($7::text = 'no_vote' AND winner IS NULL)
       OR ($7::text = 'upset' AND winner_win_chance IS NOT NULL
           AND winner_win_chance < $8))
"#;

pub(crate) async fn duels_page_for(
    state: &AppState,
    playlist_id: Uuid,
    config: &StatsConfig,
    query: DuelsQuery,
) -> Result<DuelPage, ApiError> {
    let limit = query
        .limit
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    let only = match query.only.as_deref().unwrap_or("all") {
        "voted" => "voted",
        "no_vote" => "no_vote",
        "upset" => "upset",
        _ => "all",
    };
    let sort = match query.sort.as_deref().unwrap_or("recent") {
        "upsets" => "upsets",
        _ => "recent",
    };
    let search = query.q.unwrap_or_default().trim().to_string();
    validate_text_input(&search, "search")?;
    let user_search = query.user.unwrap_or_default().trim().to_string();
    validate_text_input(&user_search, "voter")?;

    let from = query.hours.map(|hours| {
        Utc::now()
            - Duration::seconds(
                (clamped_hours(Some(hours), 24.0, 24.0 * 120.0) * 3600.0).round() as i64,
            )
    });
    let from = parse_time(query.from.as_deref(), "from")?.or(from);
    let to = parse_time(query.to.as_deref(), "to")?;

    let matching_titles: Option<Vec<Uuid>> = if search.is_empty() {
        None
    } else {
        Some(
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM tracks WHERE playlist_id = $1 AND title ILIKE '%' || $2 || '%'",
            )
            .bind(playlist_id)
            .bind(&search)
            .fetch_all(&state.db)
            .await?,
        )
    };

    let total = sqlx::query_scalar::<_, i64>(DUEL_COUNT_CTE)
        .bind(playlist_id)
        .bind(query.user_id)
        .bind(query.track_id)
        .bind(&matching_titles)
        .bind(from)
        .bind(to)
        .bind(only)
        .bind(config.upset_threshold_pct)
        .bind(&user_search)
        .fetch_one(&state.db)
        .await?;

    let (inner_order, outer_order) = if sort == "upsets" {
        (
            "winner_win_chance ASC NULLS LAST, seen_at DESC, track_low, track_high",
            "p.winner_win_chance ASC NULLS LAST, p.seen_at DESC, p.track_low, p.track_high",
        )
    } else {
        (
            "seen_at DESC, track_low, track_high",
            "p.seen_at DESC, p.track_low, p.track_high",
        )
    };
    let page_sql = DUEL_SELECT_CTE
        .replace("{inner_order}", inner_order)
        .replace("{outer_order}", outer_order);
    let rows = sqlx::query_as::<_, DuelSqlRow>(&page_sql)
        .bind(playlist_id)
        .bind(query.user_id)
        .bind(query.track_id)
        .bind(&matching_titles)
        .bind(from)
        .bind(to)
        .bind(only)
        .bind(config.upset_threshold_pct)
        .bind(&user_search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let duels = rows
        .into_iter()
        .map(|row| map_duel_row(row, config))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DuelPage {
        as_of: Utc::now(),
        total,
        limit,
        offset,
        only: only.to_string(),
        sort: sort.to_string(),
        duels,
    })
}

fn map_duel_row(row: DuelSqlRow, config: &StatsConfig) -> Result<DuelRow, ApiError> {
    let low = DuelTrack {
        id: row.track_low,
        title: row.low_title,
        artists: row.low_artists,
        release_year: row.low_release_year,
        album_image_url: row.low_image_url,
        preview_url: row.low_preview_url,
        current_rank: row.low_rank,
        current_rating: row.low_current_rating,
    };
    let high = DuelTrack {
        id: row.track_high,
        title: row.high_title,
        artists: row.high_artists,
        release_year: row.high_release_year,
        album_image_url: row.high_image_url,
        preview_url: row.high_preview_url,
        current_rank: row.high_rank,
        current_rating: row.high_current_rating,
    };

    let (track_a, track_b) = if row.served_flip {
        (high.clone(), low.clone())
    } else {
        (low.clone(), high.clone())
    };

    let (rating_a_before, rating_b_before, delta_a, delta_b, comps_a, comps_b) = if row.served_flip
    {
        (
            row.rating_high_before,
            row.rating_low_before,
            row.delta_high,
            row.delta_low,
            row.comparisons_high_before,
            row.comparisons_low_before,
        )
    } else {
        (
            row.rating_low_before,
            row.rating_high_before,
            row.delta_low,
            row.delta_high,
            row.comparisons_low_before,
            row.comparisons_high_before,
        )
    };

    let (win_chance_a, win_chance_b) = match (row.rating_low_before, row.rating_high_before) {
        (Some(low_before), Some(high_before)) => {
            let low_chance = win_chance(low_before, high_before);
            if row.served_flip {
                (Some(100.0 - low_chance), Some(low_chance))
            } else {
                (Some(low_chance), Some(100.0 - low_chance))
            }
        }
        _ => (None, None),
    };

    let (rating_a_after, rating_b_after) = match (
        row.rating_low_before,
        row.rating_high_before,
        row.delta_low,
        row.delta_high,
    ) {
        (Some(low_before), Some(high_before), Some(delta_low), Some(delta_high)) => {
            let low_after = low_before + delta_low;
            let high_after = high_before + delta_high;
            if row.served_flip {
                (Some(high_after), Some(low_after))
            } else {
                (Some(low_after), Some(high_after))
            }
        }
        _ => (None, None),
    };

    let winner_id = row.winner;
    let winner_is_low = winner_id == Some(row.track_low);
    let result_side = winner_id.map(|_| {
        winner_side(winner_is_low, row.served_flip)
            .as_str()
            .to_string()
    });
    let win_chance_pct = row.winner_win_chance;
    let upset = winner_id.is_some()
        && win_chance_pct.is_some_and(|chance| is_upset(chance, config.upset_threshold_pct));
    let rating_swing = row.delta_low.map(f64::abs);
    let idle = row
        .decision_seconds
        .is_some_and(|seconds| seconds > config.session_gap_seconds() as f64 || seconds < 0.0);
    let decision = row.decision_seconds.filter(|_| !idle);

    Ok(DuelRow {
        seen_at: row.seen_at,
        voted_at: row.voted_at,
        user_id: row.user_id,
        username: row.username,
        track_a,
        track_b,
        result_side,
        winner_id,
        no_vote: winner_id.is_none(),
        upset,
        win_chance_pct,
        rating_swing,
        decision_seconds: decision,
        idle,
        details: DuelDetails {
            served_flip: row.served_flip,
            rating_a_before,
            rating_b_before,
            rating_a_after,
            rating_b_after,
            win_chance_a,
            win_chance_b,
            rating_gap_before: match (rating_a_before, rating_b_before) {
                (Some(a), Some(b)) => Some((a - b).abs()),
                _ => None,
            },
            delta_a,
            delta_b,
            comparisons_a_before: comps_a,
            comparisons_b_before: comps_b,
            k_factor: row.k_factor,
            vote_seq: row.seq,
        },
    })
}

pub async fn duels(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<DuelsQuery>,
) -> Result<Json<DuelPage>, ApiError> {
    let _permit = heavy_permit(&state).await?;
    let playlist_id = playlist_id(&state).await?;
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;
    Ok(Json(
        duels_page_for(&state, playlist_id, &config, query).await?,
    ))
}

// ---------------------------------------------------------------------------
// Song deep dive
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct SongTrack {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album: Option<String>,
    pub release_year: Option<i32>,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub has_preview: bool,
    pub current_rank: i64,
    pub current_rating: f64,
    pub total_tracks: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongHighlight {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album_image_url: Option<String>,
    pub rating: f64,
    pub rank: i64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongRecord {
    pub wins: i64,
    pub losses: i64,
    pub voted_duels: i64,
    pub win_pct: Option<f64>,
    pub expected_wins: f64,
    pub form: f64,
    pub shown_duels: i64,
    pub no_votes: i64,
    pub no_vote_rate: Option<f64>,
    pub upset_wins: i64,
    pub upset_losses: i64,
    pub median_decision_seconds: Option<f64>,
    pub playlist_avg_duels: f64,
    pub duels: i64,
    pub has_preview: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RatingPoint {
    pub at: DateTime<Utc>,
    pub rating: f64,
    pub opponent_id: Uuid,
    pub opponent_title: String,
    pub won: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SongDetail {
    pub as_of: DateTime<Utc>,
    pub track: SongTrack,
    pub rank_24h: Option<i64>,
    pub rank_7d: Option<i64>,
    pub record: SongRecord,
    pub best_win: Option<SongHighlight>,
    pub worst_loss: Option<SongHighlight>,
    pub rating_series: Vec<RatingPoint>,
    pub history: DuelPage,
}

#[derive(Debug, sqlx::FromRow)]
struct SongSummarySql {
    voted_duels: i64,
    no_votes: i64,
    shown_duels: i64,
    wins: i64,
    expected_wins: f64,
    upset_wins: i64,
    upset_losses: i64,
    median_decision_seconds: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct SongTrackSql {
    id: Uuid,
    title: String,
    artists: String,
    album: Option<String>,
    release_year: Option<i32>,
    album_image_url: Option<String>,
    preview_url: Option<String>,
    rating: f64,
    comparisons: i32,
    current_rank: i64,
    total_tracks: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct HighlightSql {
    id: Uuid,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: f64,
    rank: i64,
    at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct RatingPointSql {
    at: DateTime<Utc>,
    rating: f64,
    opponent_id: Uuid,
    opponent_title: String,
    won: bool,
}

pub async fn song_detail(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(track_id): Path<Uuid>,
    Query(query): Query<SongDetailQuery>,
) -> Result<Json<SongDetail>, ApiError> {
    let playlist_id = playlist_id(&state).await?;
    let limit = query.limit.unwrap_or(15).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    let key = format!("admin:song:{playlist_id}:{track_id}:{limit}:{offset}");
    let value = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        song_detail_inner(&state, playlist_id, track_id, limit, offset).await
    })
    .await?;
    Ok(Json(value))
}

async fn song_detail_inner(
    state: &AppState,
    playlist_id: Uuid,
    track_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<SongDetail, ApiError> {
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;

    let track = sqlx::query_as::<_, SongTrackSql>(
        "WITH ranked AS ( \
             SELECT id, title, artists, album, release_year, album_image_url, preview_url, \
                    rating, comparisons, \
                    ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) \
                        AS current_rank, \
                    COUNT(*) OVER () AS total_tracks \
             FROM tracks WHERE playlist_id = $1 \
         ) \
         SELECT * FROM ranked WHERE id = $2",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("song not found in the active playlist".to_string()))?;

    let summary = sqlx::query_as::<_, SongSummarySql>(
        "WITH obs AS ( \
             SELECT sp.seen_at, v.winner, v.created_at, \
                    CASE WHEN v.id IS NULL THEN NULL \
                         WHEN sp.track_low = $2 \
                           THEN 1.0 / (1.0 + power(10.0::float8, \
                                (v.rating_high_before - v.rating_low_before) / 400.0)) \
                         ELSE 1.0 / (1.0 + power(10.0::float8, \
                                (v.rating_low_before - v.rating_high_before) / 400.0)) \
                    END AS track_win_chance \
             FROM seen_pairs sp \
             LEFT JOIN votes v \
               ON v.user_id = sp.user_id \
              AND v.playlist_id = sp.playlist_id \
              AND v.track_low = sp.track_low \
              AND v.track_high = sp.track_high \
             WHERE sp.playlist_id = $1 \
               AND (sp.track_low = $2 OR sp.track_high = $2) \
         ) \
         SELECT COUNT(*) FILTER (WHERE winner IS NOT NULL) AS voted_duels, \
                COUNT(*) FILTER (WHERE winner IS NULL) AS no_votes, \
                COUNT(*) AS shown_duels, \
                COUNT(*) FILTER (WHERE winner = $2) AS wins, \
                COALESCE(SUM(track_win_chance) FILTER (WHERE winner IS NOT NULL), 0)::float8 \
                    AS expected_wins, \
                COUNT(*) FILTER (WHERE winner = $2 \
                    AND track_win_chance < $3::float8 / 100.0) AS upset_wins, \
                COUNT(*) FILTER (WHERE winner IS NOT NULL AND winner <> $2 \
                    AND (1.0 - track_win_chance) < $3::float8 / 100.0) AS upset_losses, \
                percentile_cont(0.5) WITHIN GROUP ( \
                    ORDER BY EXTRACT(EPOCH FROM (created_at - seen_at)) \
                ) FILTER (WHERE winner IS NOT NULL \
                    AND created_at <= seen_at + ($4::float8 * interval '1 second')) \
                    ::float8 AS median_decision_seconds \
         FROM obs",
    )
    .bind(playlist_id)
    .bind(track_id)
    .bind(config.upset_threshold_pct)
    .bind(config.session_gap_seconds() as f64)
    .fetch_one(&state.db)
    .await?;

    let playlist_avg = sqlx::query_scalar::<_, Option<f64>>(
        "SELECT AVG(comparisons)::float8 FROM tracks WHERE playlist_id = $1",
    )
    .bind(playlist_id)
    .fetch_one(&state.db)
    .await?
    .unwrap_or(0.0);

    let best_win = load_highlight(&state.db, playlist_id, track_id, true).await?;
    let worst_loss = load_highlight(&state.db, playlist_id, track_id, false).await?;

    let rating_series = sqlx::query_as::<_, RatingPointSql>(
        "SELECT v.created_at AS at, \
                CASE WHEN v.track_low = $2 \
                     THEN v.rating_low_before + v.delta_low \
                     ELSE v.rating_high_before + v.delta_high END AS rating, \
                CASE WHEN v.track_low = $2 THEN v.track_high ELSE v.track_low END \
                    AS opponent_id, \
                opp.title AS opponent_title, \
                (v.winner = $2) AS won \
         FROM votes v \
         JOIN tracks opp ON opp.id = CASE WHEN v.track_low = $2 \
                                          THEN v.track_high ELSE v.track_low END \
         WHERE v.playlist_id = $1 AND (v.track_low = $2 OR v.track_high = $2) \
         ORDER BY v.created_at ASC, v.seq ASC \
         LIMIT 1000",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_all(&state.db)
    .await?;

    let (rank_24h, rank_7d) = load_ranks_for_track(&state.db, playlist_id, track_id).await?;

    let losses = (summary.voted_duels - summary.wins).max(0);
    let win_pct =
        (summary.voted_duels > 0).then(|| summary.wins as f64 / summary.voted_duels as f64 * 100.0);
    let no_vote_rate = (summary.shown_duels > 0)
        .then(|| summary.no_votes as f64 / summary.shown_duels as f64 * 100.0);

    let history = duels_page_for(
        state,
        playlist_id,
        &config,
        DuelsQuery {
            track_id: Some(track_id),
            limit: Some(limit),
            offset: Some(offset),
            ..DuelsQuery::default()
        },
    )
    .await?;

    Ok(SongDetail {
        as_of: Utc::now(),
        track: SongTrack {
            id: track.id,
            title: track.title,
            artists: track.artists,
            album: track.album,
            release_year: track.release_year,
            album_image_url: track.album_image_url,
            preview_url: track.preview_url.clone(),
            has_preview: track.preview_url.is_some(),
            current_rank: track.current_rank,
            current_rating: track.rating,
            total_tracks: track.total_tracks,
        },
        rank_24h,
        rank_7d,
        record: SongRecord {
            wins: summary.wins,
            losses,
            voted_duels: summary.voted_duels,
            win_pct,
            expected_wins: summary.expected_wins,
            form: summary.wins as f64 - summary.expected_wins,
            shown_duels: summary.shown_duels,
            no_votes: summary.no_votes,
            no_vote_rate,
            upset_wins: summary.upset_wins,
            upset_losses: summary.upset_losses,
            median_decision_seconds: summary.median_decision_seconds,
            playlist_avg_duels: playlist_avg,
            duels: track.comparisons as i64,
            has_preview: track.preview_url.is_some(),
        },
        best_win,
        worst_loss,
        rating_series: rating_series
            .into_iter()
            .map(|point| RatingPoint {
                at: point.at,
                rating: point.rating,
                opponent_id: point.opponent_id,
                opponent_title: point.opponent_title,
                won: point.won,
            })
            .collect(),
        history,
    })
}

#[derive(Debug, Default, Deserialize)]
pub struct SongDetailQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

async fn load_highlight(
    pool: &PgPool,
    playlist_id: Uuid,
    track_id: Uuid,
    best_win: bool,
) -> Result<Option<SongHighlight>, ApiError> {
    let order = if best_win { "DESC" } else { "ASC" };
    let filter = if best_win {
        "v.winner = $2"
    } else {
        "v.winner <> $2"
    };
    let sql = format!(
        "WITH ranked AS ( \
             SELECT id, ROW_NUMBER() OVER ( \
                 ORDER BY rating DESC, id ASC) AS rank \
             FROM tracks WHERE playlist_id = $1 \
         ) \
         SELECT opp.id, opp.title, opp.artists, opp.album_image_url, opp.rating, \
                r.rank, v.created_at AS at \
         FROM votes v \
         JOIN tracks opp ON opp.id = CASE WHEN v.track_low = $2 \
                                          THEN v.track_high ELSE v.track_low END \
         JOIN ranked r ON r.id = opp.id \
         WHERE v.playlist_id = $1 AND (v.track_low = $2 OR v.track_high = $2) \
           AND {filter} \
         ORDER BY opp.rating {order} \
         LIMIT 1"
    );
    let row = sqlx::query_as::<_, HighlightSql>(&sql)
        .bind(playlist_id)
        .bind(track_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|row| SongHighlight {
        id: row.id,
        title: row.title,
        artists: row.artists,
        album_image_url: row.album_image_url,
        rating: row.rating,
        rank: row.rank,
        at: row.at,
    }))
}

async fn load_ranks_for_track(
    pool: &PgPool,
    playlist_id: Uuid,
    track_id: Uuid,
) -> Result<(Option<i64>, Option<i64>), ApiError> {
    let timelines = load_timelines(pool, playlist_id).await?;
    let ids = sqlx::query_scalar::<_, Uuid>("SELECT id FROM tracks WHERE playlist_id = $1")
        .bind(playlist_id)
        .fetch_all(pool)
        .await?;
    let now = Utc::now();
    let ranks_24h = rank_map_at(&ids, &timelines, now - Duration::hours(24), INITIAL_RATING);
    let ranks_7d = rank_map_at(&ids, &timelines, now - Duration::days(7), INITIAL_RATING);
    Ok((
        ranks_24h.get(&track_id).copied(),
        ranks_7d.get(&track_id).copied(),
    ))
}

// ---------------------------------------------------------------------------
// Standings
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct StandingsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandingRow {
    pub rank: i64,
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub has_preview: bool,
    pub rating: f64,
    pub comparisons: i32,
    pub wins: i32,
    pub win_pct: Option<f64>,
    pub rank_change_24h: Option<i64>,
    pub form: f64,
    pub no_vote_rate: Option<f64>,
    pub last_duel_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandingsPage {
    pub as_of: DateTime<Utc>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub query: String,
    pub tracks: Vec<StandingRow>,
}

#[derive(Debug, sqlx::FromRow)]
struct StandingSql {
    rank: i64,
    id: Uuid,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    preview_url: Option<String>,
    rating: f64,
    comparisons: i32,
    wins: i32,
    expected_wins: Option<f64>,
    votes: Option<i64>,
    appearances: Option<i64>,
    last_duel_at: Option<DateTime<Utc>>,
}

const STANDING_SQL: &str = r#"
WITH ranked AS (
    SELECT id, title, artists, album_image_url, preview_url, rating, comparisons, wins,
           ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) AS rank
    FROM tracks
    WHERE playlist_id = $1
),
sides AS (
    SELECT track_low AS track_id, (winner = track_low) AS won,
           1.0 / (1.0 + power(10.0::float8,
                (rating_high_before - rating_low_before) / 400.0)) AS win_chance
    FROM votes WHERE playlist_id = $1
    UNION ALL
    SELECT track_high, (winner = track_high),
           1.0 / (1.0 + power(10.0::float8,
                (rating_low_before - rating_high_before) / 400.0))
    FROM votes WHERE playlist_id = $1
),
expected AS (
    SELECT track_id, SUM(win_chance)::float8 AS expected_wins, COUNT(*) AS votes
    FROM sides GROUP BY track_id
),
shown AS (
    SELECT track_id, COUNT(*) AS appearances, MAX(seen_at) AS last_duel_at
    FROM (
        SELECT track_low AS track_id, seen_at FROM seen_pairs WHERE playlist_id = $1
        UNION ALL
        SELECT track_high, seen_at FROM seen_pairs WHERE playlist_id = $1
    ) x
    GROUP BY track_id
)
"#;

pub async fn standings(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<StandingsQuery>,
) -> Result<Json<StandingsPage>, ApiError> {
    let playlist_id = playlist_id(&state).await?;
    let limit = query.limit.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();
    validate_text_input(&search, "search")?;

    let key = format!("admin:standings:{playlist_id}:{limit}:{offset}:{search}");
    let page = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        standings_inner(&state, playlist_id, limit, offset, search).await
    })
    .await?;
    Ok(Json(page))
}

async fn standings_inner(
    state: &AppState,
    playlist_id: Uuid,
    limit: i64,
    offset: i64,
    search: String,
) -> Result<StandingsPage, ApiError> {
    let count_sql = format!(
        "{STANDING_SQL} SELECT COUNT(*) FROM ranked \
         WHERE ($2 = '' OR title ILIKE '%' || $2 || '%' OR artists ILIKE '%' || $2 || '%')"
    );
    let total = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(playlist_id)
        .bind(&search)
        .fetch_one(&state.db)
        .await?;

    let page_sql = format!(
        "{STANDING_SQL} \
         SELECT r.rank, r.id, r.title, r.artists, r.album_image_url, r.preview_url, \
                r.rating, r.comparisons, r.wins, e.expected_wins, e.votes, \
                s.appearances, s.last_duel_at \
         FROM ranked r \
         LEFT JOIN expected e ON e.track_id = r.id \
         LEFT JOIN shown s ON s.track_id = r.id \
         WHERE ($2 = '' OR r.title ILIKE '%' || $2 || '%' \
                OR r.artists ILIKE '%' || $2 || '%') \
         ORDER BY r.rank LIMIT $3 OFFSET $4"
    );
    let rows = sqlx::query_as::<_, StandingSql>(&page_sql)
        .bind(playlist_id)
        .bind(&search)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await?;

    let timelines = load_timelines(&state.db, playlist_id).await?;
    let all_ids = sqlx::query_scalar::<_, Uuid>("SELECT id FROM tracks WHERE playlist_id = $1")
        .bind(playlist_id)
        .fetch_all(&state.db)
        .await?;
    let rank_then = rank_map_at(
        &all_ids,
        &timelines,
        Utc::now() - Duration::hours(24),
        INITIAL_RATING,
    );

    let tracks =
        rows.into_iter()
            .map(|row| {
                let win_pct =
                    (row.comparisons > 0).then(|| row.wins as f64 / row.comparisons as f64 * 100.0);
                let no_vote_rate = row.appearances.filter(|count| *count > 0).map(|count| {
                    (count - row.votes.unwrap_or(0)).max(0) as f64 / count as f64 * 100.0
                });
                StandingRow {
                    rank: row.rank,
                    id: row.id,
                    title: row.title,
                    artists: row.artists,
                    album_image_url: row.album_image_url,
                    preview_url: row.preview_url.clone(),
                    has_preview: row.preview_url.is_some(),
                    rating: row.rating,
                    comparisons: row.comparisons,
                    wins: row.wins,
                    win_pct,
                    rank_change_24h: rank_change(row.rank, rank_then.get(&row.id).copied()),
                    form: row.wins as f64 - row.expected_wins.unwrap_or(0.0),
                    no_vote_rate,
                    last_duel_at: row.last_duel_at,
                }
            })
            .collect();

    Ok(StandingsPage {
        as_of: Utc::now(),
        total,
        limit,
        offset,
        query: search,
        tracks,
    })
}

// ---------------------------------------------------------------------------
// Ledger helpers
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct LedgerRow {
    track_low: Uuid,
    track_high: Uuid,
    rating_low_before: f64,
    rating_high_before: f64,
    delta_low: f64,
    delta_high: f64,
    created_at: DateTime<Utc>,
}

/// Every vote's two per-track rating points, ordered chronologically.
pub async fn load_timelines(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<HashMap<Uuid, RatingTimeline>, ApiError> {
    let rows = sqlx::query_as::<_, LedgerRow>(
        "SELECT track_low, track_high, rating_low_before, rating_high_before, \
                delta_low, delta_high, created_at \
         FROM votes WHERE playlist_id = $1 ORDER BY created_at ASC, seq ASC",
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;

    let mut timelines: HashMap<Uuid, RatingTimeline> = HashMap::new();
    for row in rows {
        let low_after = rating_after(
            true,
            row.rating_low_before,
            row.rating_high_before,
            row.delta_low,
            row.delta_high,
        );
        let high_after = rating_after(
            false,
            row.rating_low_before,
            row.rating_high_before,
            row.delta_low,
            row.delta_high,
        );
        timelines
            .entry(row.track_low)
            .or_default()
            .push(row.created_at, low_after);
        timelines
            .entry(row.track_high)
            .or_default()
            .push(row.created_at, high_after);
    }
    Ok(timelines)
}

// ---------------------------------------------------------------------------
// Pairing health
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
pub struct PairingQuery {
    pub hours: Option<f64>,
    pub from: Option<String>,
    pub lookback_hours: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PairingBucket {
    pub key: usize,
    pub label: String,
    pub rank_min: i64,
    pub rank_max: Option<i64>,
    pub songs: i64,
    pub served: i64,
    pub served_pct: f64,
    pub target_pct: f64,
    pub unweighted_pct: f64,
    pub avg_duels_per_song: f64,
    pub duels_per_weight: f64,
}

#[derive(Debug, Serialize)]
pub struct PairingQuality {
    pub duels: i64,
    pub median_rating_gap: Option<f64>,
    pub p90_rating_gap: Option<f64>,
    pub same_bucket_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct PairingSettling {
    pub even_now: Option<f64>,
    pub even_then: Option<f64>,
    pub lookback_hours: f64,
    pub average_comparisons: f64,
    pub min_avg: f64,
    pub status: SettlingStatus,
    pub duels_left: Option<f64>,
    pub hours_left: Option<f64>,
    pub pace_per_hour: f64,
    pub bottom_share_pct: f64,
    pub bottom_target_pct: f64,
    pub warn_low_sample: bool,
}

#[derive(Debug, Serialize)]
pub struct PairingHealth {
    pub as_of: DateTime<Utc>,
    pub enabled: bool,
    pub strength: f64,
    pub scale: f64,
    pub min_avg: f64,
    pub total_tracks: i64,
    pub average_comparisons: f64,
    pub window_hours: f64,
    pub window_from: DateTime<Utc>,
    pub window_to: DateTime<Utc>,
    pub duels_in_window: i64,
    pub verdict: String,
    pub verdict_key: String,
    pub buckets: Vec<PairingBucket>,
    pub settling: PairingSettling,
    pub quality: PairingQuality,
}

#[derive(Debug, sqlx::FromRow)]
struct TrackPairRow {
    id: Uuid,
    rating: f64,
    comparisons: i32,
    rank: i64,
}

pub async fn pairing_health(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<PairingQuery>,
) -> Result<Json<PairingHealth>, ApiError> {
    let _permit = heavy_permit(&state).await?;
    let playlist_id = playlist_id(&state).await?;
    let mut connection = state.db.acquire().await?;
    let rank_config = rank_weighting_config(&mut connection).await?;

    let from = window_start(query.hours, query.from.as_deref(), 24.0)?;
    let to = Utc::now();
    let window_hours = ((to - from).num_seconds().max(1) as f64 / 3600.0).clamp(0.01, 24.0 * 120.0);
    let lookback_hours = clamped_hours(query.lookback_hours, 1.0, 24.0 * 30.0);
    let lookback_from = to - Duration::seconds((lookback_hours * 3600.0).round() as i64);

    let tracks = sqlx::query_as::<_, TrackPairRow>(
        "WITH ranked AS ( \
             SELECT id, rating, comparisons, \
                    ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) \
                        AS rank \
             FROM tracks WHERE playlist_id = $1 \
         ) SELECT * FROM ranked",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let total_tracks = tracks.len() as i64;
    let average_comparisons = if total_tracks > 0 {
        tracks
            .iter()
            .map(|track| track.comparisons as f64)
            .sum::<f64>()
            / total_tracks as f64
    } else {
        0.0
    };

    let weight = |rank: i64| {
        if rank_config.enabled {
            analytics::rank_weight(rank - 1, rank_config.strength, rank_config.scale)
        } else {
            1.0
        }
    };

    let mut bucket_songs = [0i64; 5];
    let mut bucket_weight = [0f64; 5];
    let mut bucket_duels_per_weight = [0f64; 5];
    let mut total_weight = 0.0;
    let mut ratios = Vec::with_capacity(tracks.len());
    for track in &tracks {
        let index = rank_bucket_index(track.rank);
        let weight_value = weight(track.rank);
        bucket_songs[index] += 1;
        bucket_weight[index] += weight_value;
        total_weight += weight_value;
        bucket_duels_per_weight[index] += track.comparisons as f64 / weight_value;
        ratios.push(track.comparisons as f64 / weight_value);
    }

    let appearances = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT track_id, COUNT(*) \
         FROM ( \
             SELECT track_low AS track_id FROM seen_pairs \
             WHERE playlist_id = $1 AND seen_at >= $2 AND seen_at < $3 \
             UNION ALL \
             SELECT track_high FROM seen_pairs \
             WHERE playlist_id = $1 AND seen_at >= $2 AND seen_at < $3 \
         ) x GROUP BY track_id",
    )
    .bind(playlist_id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    let duels_in_window = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM seen_pairs \
         WHERE playlist_id = $1 AND seen_at >= $2 AND seen_at < $3",
    )
    .bind(playlist_id)
    .bind(from)
    .bind(to)
    .fetch_one(&state.db)
    .await?;

    let mut bucket_served = [0i64; 5];
    let mut total_served = 0i64;
    for track in &tracks {
        let served = appearances.get(&track.id).copied().unwrap_or(0);
        bucket_served[rank_bucket_index(track.rank)] += served;
        total_served += served;
    }

    let mut buckets = Vec::with_capacity(5);
    for (index, (_, rank_min, rank_max, label)) in RANK_BUCKETS.iter().enumerate() {
        let songs = bucket_songs[index];
        let served = bucket_served[index];
        buckets.push(PairingBucket {
            key: index,
            label: (*label).to_string(),
            rank_min: *rank_min,
            rank_max: if *rank_max == i64::MAX {
                None
            } else {
                Some(*rank_max)
            },
            songs,
            served,
            served_pct: if total_served > 0 {
                served as f64 / total_served as f64 * 100.0
            } else {
                0.0
            },
            target_pct: if total_weight > 0.0 {
                bucket_weight[index] / total_weight * 100.0
            } else {
                0.0
            },
            unweighted_pct: if total_tracks > 0 {
                songs as f64 / total_tracks as f64 * 100.0
            } else {
                0.0
            },
            avg_duels_per_song: if songs > 0 {
                served as f64 / songs as f64
            } else {
                0.0
            },
            duels_per_weight: if songs > 0 {
                bucket_duels_per_weight[index] / songs as f64
            } else {
                0.0
            },
        });
    }

    // Comparisons one lookback ago, keeping today's ranks.
    let vote_appearances = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT track_id, COUNT(*) FROM ( \
             SELECT track_low AS track_id FROM votes \
             WHERE playlist_id = $1 AND created_at >= $2 \
             UNION ALL \
             SELECT track_high FROM votes \
             WHERE playlist_id = $1 AND created_at >= $2 \
         ) x GROUP BY track_id",
    )
    .bind(playlist_id)
    .bind(lookback_from)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    let mut ratios_then = Vec::with_capacity(tracks.len());
    for track in &tracks {
        let before =
            track.comparisons as f64 - vote_appearances.get(&track.id).copied().unwrap_or(0) as f64;
        let weight_value = weight(track.rank);
        ratios_then.push(before.max(0.0) / weight_value);
    }

    let even_now = evenness(&ratios);
    let even_then = evenness(&ratios_then);
    let duels_left_estimate = match (even_now, even_then) {
        (Some(now), Some(then)) => duels_left(now, then, duels_in_window as f64),
        _ => None,
    };
    let pace_per_hour = if window_hours > 0.0 {
        duels_in_window as f64 / window_hours
    } else {
        0.0
    };
    let hours_left =
        duels_left_estimate.and_then(|left| (pace_per_hour > 0.0).then_some(left / pace_per_hour));

    let bottom_share_pct = buckets
        .last()
        .map(|bucket| bucket.served_pct)
        .unwrap_or(0.0);
    let bottom_target_pct = buckets
        .last()
        .map(|bucket| bucket.target_pct)
        .unwrap_or(0.0);

    let top = buckets.first();
    let (verdict_key, verdict) = match top {
        Some(top) => {
            let top_served = top.served_pct;
            let top_target = top.target_pct;
            let top_unweighted = top.unweighted_pct;
            if (top_served - top_unweighted).abs() <= 3.0 && top_target > top_unweighted + 1.0 {
                (
                    "not_active",
                    "Weighting not active — the top bucket is served at its unweighted share.",
                )
            } else if top_served > top_target + 3.0 {
                (
                    "catching_up",
                    "Catching up — top-ranked songs are still being served more than their target share.",
                )
            } else if (top_served - top_target).abs() <= 3.0 {
                (
                    "settled",
                    "Settled — served shares are close to the rank-weighted target.",
                )
            } else {
                (
                    "spreading",
                    "Spreading — lower-ranked songs are being served a little more than target.",
                )
            }
        }
        None => ("no_data", "No tracks loaded yet."),
    };

    let quality_rows = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT track_low, track_high FROM seen_pairs \
         WHERE playlist_id = $1 AND seen_at >= $2 AND seen_at < $3",
    )
    .bind(playlist_id)
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await?;

    let rating_by_id = tracks
        .iter()
        .map(|track| (track.id, track.rating))
        .collect::<HashMap<_, _>>();
    let rank_by_id = tracks
        .iter()
        .map(|track| (track.id, track.rank))
        .collect::<HashMap<_, _>>();
    let mut gaps = Vec::with_capacity(quality_rows.len());
    let mut same_bucket = 0usize;
    for (low, high) in &quality_rows {
        if let (Some(low_rating), Some(high_rating)) =
            (rating_by_id.get(low), rating_by_id.get(high))
        {
            gaps.push((low_rating - high_rating).abs());
        }
        if let (Some(low_rank), Some(high_rank)) = (rank_by_id.get(low), rank_by_id.get(high)) {
            if rank_bucket(*low_rank) == rank_bucket(*high_rank) {
                same_bucket += 1;
            }
        }
    }

    Ok(Json(PairingHealth {
        as_of: to,
        enabled: rank_config.enabled,
        strength: rank_config.strength,
        scale: rank_config.scale,
        min_avg: rank_config.min_avg,
        total_tracks,
        average_comparisons,
        window_hours,
        window_from: from,
        window_to: to,
        duels_in_window,
        verdict: verdict.to_string(),
        verdict_key: verdict_key.to_string(),
        buckets,
        settling: PairingSettling {
            even_now,
            even_then,
            lookback_hours,
            average_comparisons,
            min_avg: rank_config.min_avg,
            status: settling_status(average_comparisons, rank_config.min_avg, even_now),
            duels_left: duels_left_estimate,
            hours_left,
            pace_per_hour,
            bottom_share_pct,
            bottom_target_pct,
            warn_low_sample: duels_in_window < 1000,
        },
        quality: PairingQuality {
            duels: quality_rows.len() as i64,
            median_rating_gap: median(&gaps),
            p90_rating_gap: percentile(&gaps, 0.9),
            same_bucket_pct: (!quality_rows.is_empty())
                .then(|| same_bucket as f64 / quality_rows.len() as f64 * 100.0),
        },
    }))
}

// ---------------------------------------------------------------------------
// Progress and ranking stability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankMove {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album_image_url: Option<String>,
    pub rank_now: i64,
    pub rank_then: i64,
    pub change: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StabilityPoint {
    pub date: String,
    pub average_abs_change: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RankingStability {
    pub top100_now: i64,
    pub top100_overlap_24h: i64,
    pub top100_overlap_7d: i64,
    pub biggest_risers_24h: Vec<RankMove>,
    pub biggest_fallers_24h: Vec<RankMove>,
    pub biggest_risers_7d: Vec<RankMove>,
    pub biggest_fallers_7d: Vec<RankMove>,
    pub average_abs_change_24h: f64,
    pub trend: Vec<StabilityPoint>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressForecast {
    pub as_of: DateTime<Utc>,
    pub duels_per_hour_24h: f64,
    pub duels_per_hour_7d: f64,
    pub total_duels: i64,
    pub total_votes: i64,
    pub avg_duels_per_song: f64,
    pub min_duels_per_song: i64,
    pub target_duels_per_song: f64,
    pub progress_pct: f64,
    pub songs_at_or_above_target: i64,
    pub total_songs: i64,
    pub scheduled_reveal_at: Option<DateTime<Utc>>,
    pub projected_total_duels: Option<f64>,
    pub projected_avg_duels_per_song: Option<f64>,
    pub target_reached_at_reveal: Option<bool>,
    pub eta_hours: Option<f64>,
    pub ranking: RankingStability,
}

#[derive(Debug, sqlx::FromRow)]
struct TrackMeta {
    id: Uuid,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: f64,
    comparisons: i32,
}

pub async fn progress_forecast(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<ProgressForecast>, ApiError> {
    let playlist = active_playlist(&state.db).await?.ok_or_else(|| {
        ApiError::NotFound("no active playlist has been imported yet".to_string())
    })?;
    let key = format!("admin:progress:{}", playlist.id);
    let value = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        progress_inner(&state, playlist).await
    })
    .await?;
    Ok(Json(value))
}

async fn progress_inner(
    state: &AppState,
    playlist: crate::models::Playlist,
) -> Result<ProgressForecast, ApiError> {
    let playlist_id = playlist.id;
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;

    let now = Utc::now();
    let counts = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        "SELECT \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1) AS total_duels, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1) AS total_votes, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1 \
             AND seen_at >= now() - interval '24 hours') AS duels_24h, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1 \
             AND seen_at >= now() - interval '7 days') AS duels_7d, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1 \
             AND created_at >= now() - interval '7 days') AS votes_7d",
    )
    .bind(playlist_id)
    .fetch_one(&state.db)
    .await?;

    let tracks = sqlx::query_as::<_, TrackMeta>(
        "SELECT id, title, artists, album_image_url, rating, comparisons \
         FROM tracks WHERE playlist_id = $1",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;
    let total_songs = tracks.len() as i64;
    let avg_duels = if total_songs > 0 {
        tracks
            .iter()
            .map(|track| track.comparisons as f64)
            .sum::<f64>()
            / total_songs as f64
    } else {
        0.0
    };
    let min_duels = tracks
        .iter()
        .map(|track| track.comparisons as i64)
        .min()
        .unwrap_or(0);
    let songs_at_or_above = tracks
        .iter()
        .filter(|track| track.comparisons as f64 >= config.target_duels_per_song)
        .count() as i64;

    let rate_24h = counts.2 as f64 / 24.0;
    let rate_7d = counts.3 as f64 / (7.0 * 24.0);

    let (projected_total, projected_avg, target_reached, eta_hours) = if let Some(reveal_at) =
        playlist.scheduled_reveal_at
    {
        let hours_until = ((reveal_at - now).num_seconds() as f64 / 3600.0).max(0.0);
        let projected_duels = counts.0 as f64 + rate_7d * hours_until;
        let comparison_rate = (counts.4 as f64 * 2.0) / (7.0 * 24.0);
        let projected_comparisons = avg_duels * total_songs as f64 + comparison_rate * hours_until;
        let projected_avg = if total_songs > 0 {
            Some(projected_comparisons / total_songs as f64)
        } else {
            None
        };
        (
            Some(projected_duels),
            projected_avg,
            projected_avg.map(|avg| avg >= config.target_duels_per_song),
            None,
        )
    } else {
        let needed = (config.target_duels_per_song * total_songs as f64
            - avg_duels * total_songs as f64)
            .max(0.0);
        let comparison_rate = (counts.4 as f64 * 2.0) / (7.0 * 24.0);
        let eta = (comparison_rate > 0.0 && needed > 0.0).then_some(needed / comparison_rate);
        (None, None, None, eta)
    };

    let ranking = load_ranking_stability(&state.db, playlist_id, &tracks, now).await?;

    Ok(ProgressForecast {
        as_of: now,
        duels_per_hour_24h: rate_24h,
        duels_per_hour_7d: rate_7d,
        total_duels: counts.0,
        total_votes: counts.1,
        avg_duels_per_song: avg_duels,
        min_duels_per_song: min_duels,
        target_duels_per_song: config.target_duels_per_song,
        progress_pct: if config.target_duels_per_song > 0.0 {
            avg_duels / config.target_duels_per_song * 100.0
        } else {
            0.0
        },
        songs_at_or_above_target: songs_at_or_above,
        total_songs,
        scheduled_reveal_at: playlist.scheduled_reveal_at,
        projected_total_duels: projected_total,
        projected_avg_duels_per_song: projected_avg,
        target_reached_at_reveal: target_reached,
        eta_hours,
        ranking,
    })
}

fn move_row(track: &TrackMeta, rank_now: i64, rank_then: i64) -> RankMove {
    RankMove {
        id: track.id,
        title: track.title.clone(),
        artists: track.artists.clone(),
        album_image_url: track.album_image_url.clone(),
        rank_now,
        rank_then,
        change: rank_then - rank_now,
    }
}

async fn load_ranking_stability(
    pool: &PgPool,
    playlist_id: Uuid,
    tracks: &[TrackMeta],
    now: DateTime<Utc>,
) -> Result<RankingStability, ApiError> {
    let timelines = load_timelines(pool, playlist_id).await?;
    let ids = tracks.iter().map(|track| track.id).collect::<Vec<_>>();
    let current = analytics::rank_map(
        &tracks
            .iter()
            .map(|track| (track.id, track.rating))
            .collect::<HashMap<_, _>>(),
    );
    let ranks_24h = rank_map_at(&ids, &timelines, now - Duration::hours(24), INITIAL_RATING);
    let ranks_7d = rank_map_at(&ids, &timelines, now - Duration::days(7), INITIAL_RATING);

    let top100_now: Vec<Uuid> = current
        .iter()
        .filter(|(_, rank)| **rank <= 100)
        .map(|(id, _)| *id)
        .collect();
    let overlap_24h = top100_now
        .iter()
        .filter(|id| ranks_24h.get(id).is_some_and(|rank| *rank <= 100))
        .count() as i64;
    let overlap_7d = top100_now
        .iter()
        .filter(|id| ranks_7d.get(id).is_some_and(|rank| *rank <= 100))
        .count() as i64;

    let mut moves_24h = tracks
        .iter()
        .filter_map(|track| {
            let now_rank = current.get(&track.id).copied()?;
            let then_rank = ranks_24h.get(&track.id).copied()?;
            (now_rank != then_rank).then(|| move_row(track, now_rank, then_rank))
        })
        .collect::<Vec<_>>();
    moves_24h.sort_by_key(|left| std::cmp::Reverse(left.change));

    let risers_24h = moves_24h
        .iter()
        .filter(|row| row.change > 0)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let fallers_24h = {
        let mut rows = moves_24h
            .iter()
            .filter(|row| row.change < 0)
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by_key(|left| left.change);
        rows.into_iter().take(5).collect::<Vec<_>>()
    };

    let mut moves_7d = tracks
        .iter()
        .filter_map(|track| {
            let now_rank = current.get(&track.id).copied()?;
            let then_rank = ranks_7d.get(&track.id).copied()?;
            (now_rank != then_rank).then(|| move_row(track, now_rank, then_rank))
        })
        .collect::<Vec<_>>();
    moves_7d.sort_by_key(|left| std::cmp::Reverse(left.change));
    let risers_7d = moves_7d
        .iter()
        .filter(|row| row.change > 0)
        .take(5)
        .cloned()
        .collect::<Vec<_>>();
    let fallers_7d = {
        let mut rows = moves_7d
            .iter()
            .filter(|row| row.change < 0)
            .cloned()
            .collect::<Vec<_>>();
        rows.sort_by_key(|left| left.change);
        rows.into_iter().take(5).collect::<Vec<_>>()
    };

    let average_abs_change_24h = top100_now
        .iter()
        .filter_map(|id| {
            let now_rank = current.get(id)?;
            let then_rank = ranks_24h.get(id)?;
            Some((now_rank - then_rank).abs() as f64)
        })
        .sum::<f64>()
        / top100_now.len().max(1) as f64;

    let mut trend = Vec::with_capacity(14);
    for days_ago in 0..14 {
        let at = now - Duration::days(days_ago);
        let previous = now - Duration::days(days_ago + 1);
        let snapshot = rank_map_at(&ids, &timelines, at, INITIAL_RATING);
        let prior = rank_map_at(&ids, &timelines, previous, INITIAL_RATING);
        let top: Vec<Uuid> = snapshot
            .iter()
            .filter(|(_, rank)| **rank <= 100)
            .map(|(id, _)| *id)
            .collect();
        let average = top
            .iter()
            .filter_map(|id| {
                let now_rank = snapshot.get(id)?;
                let then_rank = prior.get(id)?;
                Some((now_rank - then_rank).abs() as f64)
            })
            .sum::<f64>()
            / top.len().max(1) as f64;
        trend.push(StabilityPoint {
            date: at.format("%Y-%m-%d").to_string(),
            average_abs_change: average,
        });
    }
    trend.reverse();

    Ok(RankingStability {
        top100_now: top100_now.len() as i64,
        top100_overlap_24h: overlap_24h,
        top100_overlap_7d: overlap_7d,
        biggest_risers_24h: risers_24h,
        biggest_fallers_24h: fallers_24h,
        biggest_risers_7d: risers_7d,
        biggest_fallers_7d: fallers_7d,
        average_abs_change_24h,
        trend,
    })
}

// ---------------------------------------------------------------------------
// Song-level insights
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TrackBrief {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album_image_url: Option<String>,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct PreviewImpact {
    pub has_preview: bool,
    pub songs: i64,
    pub no_vote_rate: Option<f64>,
    pub win_pct: Option<f64>,
    pub average_rank: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct FormRow {
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album_image_url: Option<String>,
    pub votes: i64,
    pub wins: i64,
    pub expected_wins: f64,
    pub form: f64,
}

#[derive(Debug, Serialize)]
pub struct HistogramBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SongInsights {
    pub as_of: DateTime<Utc>,
    pub most_no_voted: Vec<TrackBrief>,
    pub preview_impact: Vec<PreviewImpact>,
    pub overperformers: Vec<FormRow>,
    pub underperformers: Vec<FormRow>,
    pub never_voted_count: i64,
    pub never_voted: Vec<TrackBrief>,
    pub fewest_duels: Vec<TrackBrief>,
    pub rating_histogram: Vec<HistogramBucket>,
    pub duels_histogram: Vec<HistogramBucket>,
}

pub async fn song_insights(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<SongInsights>, ApiError> {
    let _permit = heavy_permit(&state).await?;
    let playlist_id = playlist_id(&state).await?;

    let most_no_voted = sqlx::query_as::<_, (Uuid, String, String, Option<String>, f64)>(
        "WITH shown AS ( \
             SELECT track_id, COUNT(*) AS appearances FROM ( \
                 SELECT track_low AS track_id FROM seen_pairs WHERE playlist_id = $1 \
                 UNION ALL \
                 SELECT track_high FROM seen_pairs WHERE playlist_id = $1 \
             ) x GROUP BY track_id \
         ), voted AS ( \
             SELECT track_id, COUNT(*) AS votes FROM ( \
                 SELECT track_low AS track_id FROM votes WHERE playlist_id = $1 \
                 UNION ALL \
                 SELECT track_high FROM votes WHERE playlist_id = $1 \
             ) x GROUP BY track_id \
         ) \
         SELECT t.id, t.title, t.artists, t.album_image_url, \
                (s.appearances - COALESCE(v.votes, 0))::float8 / s.appearances AS rate \
         FROM tracks t \
         JOIN shown s ON s.track_id = t.id \
         LEFT JOIN voted v ON v.track_id = t.id \
         WHERE t.playlist_id = $1 AND s.appearances >= 10 \
         ORDER BY rate DESC, s.appearances DESC LIMIT 10",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, title, artists, image, value)| TrackBrief {
        id,
        title,
        artists,
        album_image_url: image,
        value: value * 100.0,
    })
    .collect::<Vec<_>>();

    let preview_rows = sqlx::query_as::<_, (bool, i64, Option<f64>, Option<f64>, Option<f64>)>(
        "WITH ranked AS ( \
             SELECT id, (preview_url IS NOT NULL) AS has_preview, rating, comparisons, wins, \
                    ROW_NUMBER() OVER (ORDER BY rating DESC, id ASC) AS rank \
             FROM tracks WHERE playlist_id = $1 \
         ), shown AS ( \
             SELECT track_id, COUNT(*) AS appearances FROM ( \
                 SELECT track_low AS track_id FROM seen_pairs WHERE playlist_id = $1 \
                 UNION ALL \
                 SELECT track_high FROM seen_pairs WHERE playlist_id = $1 \
             ) x GROUP BY track_id \
         ), voted AS ( \
             SELECT track_id, COUNT(*) AS votes FROM ( \
                 SELECT track_low AS track_id FROM votes WHERE playlist_id = $1 \
                 UNION ALL \
                 SELECT track_high FROM votes WHERE playlist_id = $1 \
             ) x GROUP BY track_id \
         ) \
         SELECT r.has_preview, COUNT(*) AS songs, \
                SUM((s.appearances - COALESCE(v.votes, 0))::float8) \
                    / NULLIF(SUM(s.appearances), 0)::float8 * 100 AS no_vote_rate, \
                SUM(r.wins)::float8 / NULLIF(SUM(r.comparisons), 0)::float8 * 100 AS win_pct, \
                AVG(r.rank)::float8 AS average_rank \
         FROM ranked r \
         LEFT JOIN shown s ON s.track_id = r.id \
         LEFT JOIN voted v ON v.track_id = r.id \
         GROUP BY r.has_preview ORDER BY r.has_preview DESC",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let preview_impact = preview_rows
        .into_iter()
        .map(
            |(has_preview, songs, no_vote_rate, win_pct, average_rank)| PreviewImpact {
                has_preview,
                songs,
                no_vote_rate,
                win_pct,
                average_rank,
            },
        )
        .collect::<Vec<_>>();

    let over_rows = load_form_rows(&state.db, playlist_id, false).await?;
    let under_rows = load_form_rows(&state.db, playlist_id, true).await?;

    let never_voted_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tracks WHERE playlist_id = $1 AND comparisons = 0",
    )
    .bind(playlist_id)
    .fetch_one(&state.db)
    .await?;

    let never_voted = sqlx::query_as::<_, (Uuid, String, String, Option<String>, i32)>(
        "SELECT id, title, artists, album_image_url, comparisons FROM tracks \
         WHERE playlist_id = $1 AND comparisons = 0 \
         ORDER BY title ASC LIMIT 20",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, title, artists, image, comparisons)| TrackBrief {
        id,
        title,
        artists,
        album_image_url: image,
        value: comparisons as f64,
    })
    .collect::<Vec<_>>();

    let fewest_duels = sqlx::query_as::<_, (Uuid, String, String, Option<String>, i32)>(
        "SELECT id, title, artists, album_image_url, comparisons FROM tracks \
         WHERE playlist_id = $1 ORDER BY comparisons ASC, title ASC LIMIT 20",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, title, artists, image, comparisons)| TrackBrief {
        id,
        title,
        artists,
        album_image_url: image,
        value: comparisons as f64,
    })
    .collect::<Vec<_>>();

    let all_tracks = sqlx::query_as::<_, (f64, i32)>(
        "SELECT rating, comparisons FROM tracks WHERE playlist_id = $1",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let rating_labels = [
        "<900",
        "900–949",
        "950–999",
        "1000–1049",
        "1050–1099",
        "1100–1149",
        "1150–1199",
        "1200+",
    ];
    let mut rating_counts = [0i64; 8];
    for (rating, _) in &all_tracks {
        let index = if *rating < 900.0 {
            0
        } else if *rating < 950.0 {
            1
        } else if *rating < 1000.0 {
            2
        } else if *rating < 1050.0 {
            3
        } else if *rating < 1100.0 {
            4
        } else if *rating < 1150.0 {
            5
        } else if *rating < 1200.0 {
            6
        } else {
            7
        };
        rating_counts[index] += 1;
    }
    let rating_histogram = rating_labels
        .iter()
        .zip(rating_counts)
        .map(|(label, count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    let duel_labels = ["0", "1–5", "6–10", "11–20", "21–50", "51+"];
    let mut duel_counts = [0i64; 6];
    for (_, comparisons) in &all_tracks {
        let index = match *comparisons {
            0 => 0,
            1..=5 => 1,
            6..=10 => 2,
            11..=20 => 3,
            21..=50 => 4,
            _ => 5,
        };
        duel_counts[index] += 1;
    }
    let duels_histogram = duel_labels
        .iter()
        .zip(duel_counts)
        .map(|(label, count)| HistogramBucket {
            label: (*label).to_string(),
            count,
        })
        .collect::<Vec<_>>();

    Ok(Json(SongInsights {
        as_of: Utc::now(),
        most_no_voted,
        preview_impact,
        overperformers: form_to_rows(over_rows),
        underperformers: form_to_rows(under_rows),
        never_voted_count,
        never_voted,
        fewest_duels,
        rating_histogram,
        duels_histogram,
    }))
}

type FormSqlRow = (Uuid, String, String, Option<String>, i64, i64, f64, f64);

async fn load_form_rows(
    pool: &PgPool,
    playlist_id: Uuid,
    ascending: bool,
) -> Result<Vec<FormSqlRow>, ApiError> {
    let order = if ascending { "ASC" } else { "DESC" };
    let sql = format!(
        "WITH sides AS ( \
             SELECT track_low AS track_id, (winner = track_low) AS won, \
                    1.0 / (1.0 + power(10.0::float8, \
                        (rating_high_before - rating_low_before) / 400.0)) AS win_chance \
             FROM votes WHERE playlist_id = $1 \
             UNION ALL \
             SELECT track_high, (winner = track_high), \
                    1.0 / (1.0 + power(10.0::float8, \
                        (rating_low_before - rating_high_before) / 400.0)) \
             FROM votes WHERE playlist_id = $1 \
         ), agg AS ( \
             SELECT track_id, COUNT(*) AS votes, \
                    COUNT(*) FILTER (WHERE won) AS wins, \
                    SUM(win_chance)::float8 AS expected_wins \
             FROM sides GROUP BY track_id \
         ) \
         SELECT t.id, t.title, t.artists, t.album_image_url, a.votes, a.wins, \
                a.expected_wins, (a.wins - a.expected_wins)::float8 AS form \
         FROM agg a JOIN tracks t ON t.id = a.track_id \
         WHERE a.votes >= 10 \
         ORDER BY form {order} LIMIT 10"
    );
    Ok(
        sqlx::query_as::<_, (Uuid, String, String, Option<String>, i64, i64, f64, f64)>(&sql)
            .bind(playlist_id)
            .fetch_all(pool)
            .await?,
    )
}

fn form_to_rows(rows: Vec<FormSqlRow>) -> Vec<FormRow> {
    rows.into_iter()
        .map(
            |(id, title, artists, image, votes, wins, expected_wins, form)| FormRow {
                id,
                title,
                artists,
                album_image_url: image,
                votes,
                wins,
                expected_wins,
                form,
            },
        )
        .collect()
}

// ---------------------------------------------------------------------------
// Vote quality overview
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct FlagCount {
    pub voters: i64,
    pub vote_share_pct: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VoteQuality {
    pub as_of: DateTime<Utc>,
    pub total_votes: i64,
    pub total_duels: i64,
    pub no_vote_rate: Option<f64>,
    pub upset_rate: Option<f64>,
    pub upset_threshold_pct: f64,
    pub median_decision_seconds: Option<f64>,
    pub p90_decision_seconds: Option<f64>,
    pub idle_excluded: i64,
    pub left_win_pct: Option<f64>,
    pub right_win_pct: Option<f64>,
    pub flagged_voters: i64,
    pub flagged_vote_share_pct: f64,
    pub flag_counts: HashMap<String, FlagCount>,
    pub weekday_hour: Heatmap,
}

pub async fn vote_quality(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<VoteQualityQuery>,
) -> Result<Json<VoteQuality>, ApiError> {
    let playlist_id = playlist_id(&state).await?;
    let key = format!(
        "admin:vote-quality:{playlist_id}:{}",
        query.tz.clone().unwrap_or_default()
    );
    let value = cached(&state, key, 45, || async {
        let _permit = heavy_permit(&state).await?;
        vote_quality_inner(&state, playlist_id, query.tz.as_deref()).await
    })
    .await?;
    Ok(Json(value))
}

async fn vote_quality_inner(
    state: &AppState,
    playlist_id: Uuid,
    requested_tz: Option<&str>,
) -> Result<VoteQuality, ApiError> {
    let mut connection = state.db.acquire().await?;
    let config = stats_config(&mut connection).await?;
    let timezone = resolve_timezone(&mut connection, requested_tz).await?;

    let row = sqlx::query_as::<_, (i64, i64, i64, Option<f64>, Option<f64>, i64, i64, i64, i64)>(
        "SELECT COUNT(*) AS total_votes, \
                COUNT(*) FILTER (WHERE winner_win_chance < $2) AS upsets, \
                COUNT(*) FILTER (WHERE decision_seconds > $3) AS idle_excluded, \
                percentile_cont(0.5) WITHIN GROUP (ORDER BY decision_seconds) \
                    FILTER (WHERE decision_seconds <= $3)::float8 AS median_decision, \
                percentile_cont(0.9) WITHIN GROUP (ORDER BY decision_seconds) \
                    FILTER (WHERE decision_seconds <= $3)::float8 AS p90_decision, \
                COUNT(*) FILTER (WHERE winner_is_a) AS left_wins, \
                COUNT(*) FILTER (WHERE NOT winner_is_a) AS right_wins, \
                (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1) AS total_duels, \
                COUNT(*) AS counted \
         FROM ( \
             SELECT CASE WHEN sp.served_flip THEN v.winner = sp.track_high \
                         ELSE v.winner = sp.track_low END AS winner_is_a, \
                    CASE WHEN v.winner = sp.track_low \
                         THEN 100.0 / (1.0 + power(10.0::float8, \
                              (v.rating_high_before - v.rating_low_before) / 400.0)) \
                         ELSE 100.0 / (1.0 + power(10.0::float8, \
                              (v.rating_low_before - v.rating_high_before) / 400.0)) \
                    END AS winner_win_chance, \
                    EXTRACT(EPOCH FROM (v.created_at - sp.seen_at))::float8 \
                        AS decision_seconds \
             FROM votes v \
             JOIN seen_pairs sp \
               ON sp.user_id = v.user_id \
              AND sp.playlist_id = v.playlist_id \
              AND sp.track_low = v.track_low \
              AND sp.track_high = v.track_high \
             WHERE v.playlist_id = $1 \
         ) obs",
    )
    .bind(playlist_id)
    .bind(config.upset_threshold_pct)
    .bind(config.session_gap_seconds() as f64)
    .fetch_one(&state.db)
    .await?;

    let heatmap_rows = sqlx::query_as::<_, (i32, i32, i64)>(
        "SELECT EXTRACT(DOW FROM sp.seen_at AT TIME ZONE $2)::int AS dow, \
                EXTRACT(HOUR FROM sp.seen_at AT TIME ZONE $2)::int AS hour, \
                COUNT(*) \
         FROM votes v \
         JOIN seen_pairs sp \
           ON sp.user_id = v.user_id \
          AND sp.playlist_id = v.playlist_id \
          AND sp.track_low = v.track_low \
          AND sp.track_high = v.track_high \
         WHERE v.playlist_id = $1 \
         GROUP BY dow, hour",
    )
    .bind(playlist_id)
    .bind(&timezone)
    .fetch_all(&state.db)
    .await?;

    let mut cells = vec![vec![0i64; 24]; 7];
    for (dow, hour, count) in heatmap_rows {
        let dow = dow.clamp(0, 6) as usize;
        let hour = hour.clamp(0, 23) as usize;
        cells[dow][hour] = count;
    }

    let (flagged_voters, flagged_vote_share, flag_counts) =
        crate::api::admin_voters::flag_summary(state, playlist_id, &config).await?;

    let total_votes = row.0;
    let total_duels = row.7;
    let no_vote_rate = (total_duels > 0)
        .then(|| (total_duels - total_votes).max(0) as f64 / total_duels as f64 * 100.0);
    let upset_rate = (total_votes > 0).then(|| row.1 as f64 / total_votes as f64 * 100.0);
    let total_wins = row.5 + row.6;
    let left_win_pct = (total_wins > 0).then(|| row.5 as f64 / total_wins as f64 * 100.0);
    let right_win_pct = (total_wins > 0).then(|| row.6 as f64 / total_wins as f64 * 100.0);

    Ok(VoteQuality {
        as_of: Utc::now(),
        total_votes,
        total_duels,
        no_vote_rate,
        upset_rate,
        upset_threshold_pct: config.upset_threshold_pct,
        median_decision_seconds: row.3,
        p90_decision_seconds: row.4,
        idle_excluded: row.2,
        left_win_pct,
        right_win_pct,
        flagged_voters,
        flagged_vote_share_pct: flagged_vote_share,
        flag_counts,
        weekday_hour: Heatmap::new(cells),
    })
}

#[derive(Debug, Default, Deserialize)]
pub struct VoteQualityQuery {
    pub tz: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn config() -> StatsConfig {
        StatsConfig {
            session_gap_minutes: 15,
            fast_decision_seconds: 1.5,
            upset_threshold_pct: 35.0,
            side_bias_margin_pct: 15.0,
            low_agreement_pct: 55.0,
            min_votes_for_flags: 30,
            target_duels_per_song: 30.0,
        }
    }

    fn duel_row(flip: bool, winner: Option<Uuid>) -> DuelSqlRow {
        DuelSqlRow {
            user_id: Uuid::from_u128(9),
            username: "voter".to_string(),
            seen_at: Utc::now(),
            served_flip: flip,
            track_low: Uuid::from_u128(1),
            track_high: Uuid::from_u128(2),
            winner,
            voted_at: Some(Utc::now()),
            rating_low_before: Some(1000.0),
            rating_high_before: Some(1200.0),
            delta_low: Some(20.0),
            delta_high: Some(-20.0),
            comparisons_low_before: Some(4),
            comparisons_high_before: Some(8),
            k_factor: Some(39.5),
            seq: Some(77),
            low_title: "Low".to_string(),
            low_artists: "Low Artist".to_string(),
            low_release_year: Some(1990),
            low_image_url: Some("low.jpg".to_string()),
            low_preview_url: Some("low.mp3".to_string()),
            high_title: "High".to_string(),
            high_artists: "High Artist".to_string(),
            high_release_year: Some(2000),
            high_image_url: Some("high.jpg".to_string()),
            high_preview_url: Some("high.mp3".to_string()),
            low_rank: 2,
            high_rank: 1,
            low_current_rating: 1020.0,
            high_current_rating: 1180.0,
            winner_win_chance: Some(24.0),
            decision_seconds: Some(5.0),
        }
    }

    #[test]
    fn parse_time_accepts_rfc3339_and_normalises_to_utc() {
        assert_eq!(
            parse_time(Some("2025-01-02T03:04:05+02:00"), "from").unwrap(),
            Some(Utc.with_ymd_and_hms(2025, 1, 2, 1, 4, 5).unwrap())
        );
    }

    #[test]
    fn parse_time_treats_empty_values_as_missing() {
        assert_eq!(parse_time(None, "from").unwrap(), None);
        assert_eq!(parse_time(Some(""), "from").unwrap(), None);
        assert_eq!(parse_time(Some("   "), "from").unwrap(), None);
    }

    #[test]
    fn parse_time_rejects_invalid_timestamps() {
        let error = parse_time(Some("yesterday"), "to").unwrap_err();
        assert!(matches!(error, ApiError::BadRequest(message) if message.contains("to")));
    }

    #[test]
    fn parse_time_rejects_control_characters() {
        let error = parse_time(Some("2025-01-01T00:00:00Z\0"), "from").unwrap_err();
        assert!(
            matches!(error, ApiError::BadRequest(message) if message.contains("invalid characters"))
        );
        // Leading/trailing whitespace is intentionally trimmed by the query
        // parser before validation.
        assert!(parse_time(Some(" 2025-01-01T00:00:00Z\n"), "from").is_ok());
    }

    #[test]
    fn window_start_prefers_an_explicit_from() {
        let explicit = Utc.with_ymd_and_hms(2025, 5, 1, 0, 0, 0).unwrap();
        let start = window_start(Some(9999.0), Some("2025-05-01T00:00:00Z"), 24.0).unwrap();
        assert_eq!(start, explicit);
    }

    #[test]
    fn window_start_clamps_hours_into_the_supported_range() {
        let before = Utc::now();
        let start = window_start(Some(0.0), None, 24.0).unwrap();
        let after = Utc::now();
        assert!(start <= before - Duration::seconds(35));
        assert!(start >= after - Duration::seconds(37));

        let far = window_start(Some(f64::INFINITY), None, 24.0).unwrap();
        assert!(far <= Utc::now() - Duration::hours(24 - 1));
        assert!(far >= Utc::now() - Duration::hours(24 + 1));
    }

    #[test]
    fn window_start_uses_the_default_for_non_finite_hours() {
        let start = window_start(Some(f64::NAN), None, 24.0).unwrap();
        let expected = Utc::now() - Duration::hours(24);
        assert!((start - expected).num_seconds().abs() < 5);
    }

    #[test]
    fn clamped_hours_guards_every_non_finite_input() {
        assert_eq!(clamped_hours(Some(f64::NAN), 12.0, 100.0), 12.0);
        assert_eq!(clamped_hours(Some(f64::INFINITY), 12.0, 100.0), 12.0);
        assert_eq!(clamped_hours(Some(f64::NEG_INFINITY), 12.0, 100.0), 12.0);
        assert_eq!(clamped_hours(None, 12.0, 100.0), 12.0);
        assert_eq!(clamped_hours(Some(-5.0), 12.0, 100.0), 0.01);
        assert_eq!(clamped_hours(Some(500.0), 12.0, 100.0), 100.0);
    }

    #[test]
    fn map_duel_row_keeps_the_normal_orientation() {
        let row = map_duel_row(duel_row(false, Some(Uuid::from_u128(1))), &config()).unwrap();
        assert_eq!(row.track_a.title, "Low");
        assert_eq!(row.track_b.title, "High");
        assert_eq!(row.result_side.as_deref(), Some("A"));
        assert_eq!(row.details.rating_a_before, Some(1000.0));
        assert_eq!(row.details.rating_b_before, Some(1200.0));
        assert_eq!(row.details.rating_a_after, Some(1020.0));
        assert_eq!(row.details.rating_b_after, Some(1180.0));
        assert_eq!(row.details.delta_a, Some(20.0));
        assert_eq!(row.details.delta_b, Some(-20.0));
        assert_eq!(row.details.comparisons_a_before, Some(4));
        assert_eq!(row.details.comparisons_b_before, Some(8));
        assert!((row.details.rating_gap_before.unwrap() - 200.0).abs() < 1e-9);
        assert_eq!(row.details.k_factor, Some(39.5));
        assert_eq!(row.details.vote_seq, Some(77));
    }

    #[test]
    fn map_duel_row_swaps_everything_when_the_sides_were_flipped() {
        let row = map_duel_row(duel_row(true, Some(Uuid::from_u128(1))), &config()).unwrap();
        assert_eq!(row.track_a.title, "High");
        assert_eq!(row.track_b.title, "Low");
        assert_eq!(row.result_side.as_deref(), Some("B"));
        assert_eq!(row.details.rating_a_before, Some(1200.0));
        assert_eq!(row.details.rating_b_before, Some(1000.0));
        assert_eq!(row.details.rating_a_after, Some(1180.0));
        assert_eq!(row.details.rating_b_after, Some(1020.0));
        assert_eq!(row.details.delta_a, Some(-20.0));
        assert_eq!(row.details.delta_b, Some(20.0));
        assert_eq!(row.details.comparisons_a_before, Some(8));
        assert_eq!(row.details.comparisons_b_before, Some(4));
    }

    #[test]
    fn map_duel_row_reports_win_chances_in_served_order() {
        let normal = map_duel_row(duel_row(false, None), &config()).unwrap();
        assert!((normal.details.win_chance_a.unwrap() - 24.0253073352).abs() < 1e-6);
        assert!((normal.details.win_chance_b.unwrap() - 75.9746926648).abs() < 1e-6);

        let flipped = map_duel_row(duel_row(true, None), &config()).unwrap();
        assert!((flipped.details.win_chance_a.unwrap() - 75.9746926648).abs() < 1e-6);
        assert!((flipped.details.win_chance_b.unwrap() - 24.0253073352).abs() < 1e-6);
    }

    #[test]
    fn map_duel_row_marks_upsets_from_the_winner_chance() {
        let upset = map_duel_row(duel_row(false, Some(Uuid::from_u128(1))), &config()).unwrap();
        assert!(upset.upset);

        let mut expected = duel_row(false, Some(Uuid::from_u128(2)));
        expected.winner_win_chance = Some(75.0);
        assert!(!map_duel_row(expected, &config()).unwrap().upset);
    }

    #[test]
    fn map_duel_row_treats_a_missing_vote_as_no_vote() {
        let row = map_duel_row(duel_row(false, None), &config()).unwrap();
        assert!(row.no_vote);
        assert_eq!(row.result_side, None);
        assert_eq!(row.winner_id, None);
        assert!(!row.upset);
    }

    #[test]
    fn map_duel_row_keeps_a_normal_decision_time() {
        let row = map_duel_row(duel_row(false, Some(Uuid::from_u128(1))), &config()).unwrap();
        assert_eq!(row.decision_seconds, Some(5.0));
        assert!(!row.idle);
    }

    #[test]
    fn map_duel_row_excludes_idle_and_negative_decisions() {
        let mut idle = duel_row(false, Some(Uuid::from_u128(1)));
        idle.decision_seconds = Some(901.0);
        let row = map_duel_row(idle, &config()).unwrap();
        assert!(row.idle);
        assert_eq!(row.decision_seconds, None);

        let mut negative = duel_row(false, Some(Uuid::from_u128(1)));
        negative.decision_seconds = Some(-0.1);
        let row = map_duel_row(negative, &config()).unwrap();
        assert!(row.idle);
        assert_eq!(row.decision_seconds, None);
    }

    #[test]
    fn map_duel_row_handles_missing_rating_history() {
        let mut input = duel_row(false, Some(Uuid::from_u128(1)));
        input.rating_low_before = None;
        input.rating_high_before = None;
        input.delta_low = None;
        input.delta_high = None;
        let row = map_duel_row(input, &config()).unwrap();
        assert_eq!(row.details.rating_a_before, None);
        assert_eq!(row.details.rating_b_before, None);
        assert_eq!(row.details.rating_a_after, None);
        assert_eq!(row.details.rating_b_after, None);
        assert_eq!(row.details.win_chance_a, None);
        assert_eq!(row.details.win_chance_b, None);
        assert_eq!(row.details.rating_gap_before, None);
        assert_eq!(row.rating_swing, None);
    }

    #[test]
    fn map_duel_row_uses_the_absolute_rating_swing() {
        let mut input = duel_row(false, Some(Uuid::from_u128(2)));
        input.delta_low = Some(-7.5);
        let row = map_duel_row(input, &config()).unwrap();
        assert_eq!(row.rating_swing, Some(7.5));
        assert_eq!(row.result_side.as_deref(), Some("B"));
    }

    fn track_meta() -> TrackMeta {
        TrackMeta {
            id: Uuid::from_u128(1),
            title: "Title".to_string(),
            artists: "Artist".to_string(),
            album_image_url: Some("image".to_string()),
            rating: 1000.0,
            comparisons: 3,
        }
    }

    #[test]
    fn move_row_calculates_a_climb() {
        let move_row = move_row(&track_meta(), 5, 20);
        assert_eq!(move_row.id, Uuid::from_u128(1));
        assert_eq!(move_row.rank_now, 5);
        assert_eq!(move_row.rank_then, 20);
        assert_eq!(move_row.change, 15);
    }

    #[test]
    fn move_row_calculates_a_fall() {
        let move_row = move_row(&track_meta(), 20, 5);
        assert_eq!(move_row.change, -15);
    }

    #[test]
    fn form_rows_convert_tuple_columns() {
        let rows = vec![(
            Uuid::from_u128(1),
            "Title".to_string(),
            "Artist".to_string(),
            Some("image".to_string()),
            10,
            7,
            5.5,
            1.5,
        )];
        let converted = form_to_rows(rows);
        assert_eq!(converted.len(), 1);
        assert_eq!(converted[0].id, Uuid::from_u128(1));
        assert_eq!(converted[0].votes, 10);
        assert_eq!(converted[0].wins, 7);
        assert_eq!(converted[0].expected_wins, 5.5);
        assert_eq!(converted[0].form, 1.5);
    }

    #[test]
    fn form_rows_convert_an_empty_list() {
        assert!(form_to_rows(Vec::new()).is_empty());
    }
}
