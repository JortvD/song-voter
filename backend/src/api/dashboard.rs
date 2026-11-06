use std::fmt::Write as _;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
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
    sort: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AdminUsersPage {
    total: i64,
    limit: i64,
    offset: i64,
    query: String,
    sort: String,
    users: Vec<AdminUserRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdminUserRow {
    id: Uuid,
    username: String,
    is_admin: bool,
    disabled_at: Option<DateTime<Utc>>,
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

#[derive(Debug, Deserialize)]
pub struct FullResetRequest {
    confirm: String,
}

#[derive(Debug, Serialize)]
pub struct FullResetResponse {
    reset: bool,
    deleted_users: u64,
    deleted_invites: u64,
    deleted_playlists: u64,
    deleted_tracks: u64,
    deleted_votes: u64,
    deleted_duels: u64,
    deleted_artists: u64,
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
        Some(playlist) => (Some(playlist.name), playlist.reveal_state == "revealed"),
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
    crate::error::validate_text_input(&search, "search")?;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users \
         WHERE ($1 = '' OR username ILIKE '%' || $1 || '%')",
    )
    .bind(&search)
    .fetch_one(&state.db)
    .await?;

    let (sort_key, order_by) = match query.sort.as_deref().unwrap_or("last_active") {
        "picks" => ("picks", "COALESCE(v.votes, 0) DESC, u.username ASC"),
        "duels" => ("duels", "COALESCE(s.matchups, 0) DESC, u.username ASC"),
        "joined" => ("joined", "u.created_at DESC, u.username ASC"),
        "username" => ("username", "u.username ASC"),
        _ => (
            "last_active",
            "COALESCE(v.last_vote_at, s.last_seen_at, u.created_at) DESC, u.created_at DESC",
        ),
    };

    let sql = format!(
        "SELECT u.id, u.username, u.is_admin, u.disabled_at, u.created_at, \
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
         ORDER BY {order_by} \
         LIMIT $3 OFFSET $4"
    );

    let users = sqlx::query_as::<_, AdminUserRow>(&sql)
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
        sort: sort_key.to_string(),
        users,
    }))
}

/// Every voter, with picks and activity, as a CSV download for mission control.
pub async fn users_csv(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Response, ApiError> {
    let playlist = active_playlist(&state.db).await?;
    let playlist_id = playlist.as_ref().map(|playlist| playlist.id);

    let rows = sqlx::query_as::<_, AdminUserRow>(
        "SELECT u.id, u.username, u.is_admin, u.disabled_at, u.created_at, \
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
         ORDER BY COALESCE(v.last_vote_at, s.last_seen_at, u.created_at) DESC, \
                  u.created_at DESC",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from("username,is_admin,picks,duels_seen,joined_at,last_active_at\n");

    for row in &rows {
        let _ = writeln!(
            &mut csv,
            "{},{},{},{},{},{}",
            crate::api::chart::csv_field(&row.username),
            row.is_admin,
            row.votes_cast,
            row.matchups_seen,
            row.created_at.to_rfc3339(),
            row.last_active_at.to_rfc3339()
        );
    }

    tracing::info!(
        admin_id = %admin.0.id,
        rows = rows.len(),
        "voters CSV exported"
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"playlist-voter-voters.csv\"",
        )
        .body(Body::from(csv))
        .map_err(ApiError::internal)
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
    crate::error::validate_text_input(&search, "search")?;
    let genre = query
        .genre
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(genre) = genre.as_deref() {
        crate::error::validate_text_input(genre, "genre")?;
    }

    Ok(Json(
        load_chart(
            &state.db,
            &playlist,
            limit,
            offset,
            &search,
            query.decade,
            genre.as_deref(),
            query.artist,
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
pub struct InsightsQuery {
    days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AdminInsights {
    playlist_id: Option<Uuid>,
    playlist_name: Option<String>,
    playlist_image_url: Option<String>,
    revealed: bool,
    range_days: i64,
    range_start: String,
    range_end: String,
    range_picks: i64,
    range_voters: i64,
    range_duels: i64,
    range_picks_per_voter: f64,
    range_duels_per_voter: f64,
    total_tracks: i64,
    tracks_compared: i64,
    tracks_uncompared: i64,
    previews_missing: i64,
    total_picks: i64,
    picks_today: i64,
    picks_7d: i64,
    picks_per_voter: f64,
    total_voters: i64,
    active_voters_24h: i64,
    active_voters_7d: i64,
    total_users: i64,
    admins: i64,
    invites_total: i64,
    invites_used: i64,
    invites_open: i64,
    invites_expired: i64,
    total_duels: i64,
    duels_per_voter: f64,
    coverage_pct: f64,
    daily: Vec<InsightDaily>,
    hourly: Vec<HourlyPicks>,
    pick_distribution: Vec<PickBucket>,
    top_tracks: Vec<TopTrackRow>,
    top_voters: Vec<TopVoterRow>,
    recent_picks: Vec<RecentPickRow>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InsightDaily {
    date: String,
    votes: i64,
    voters: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct HourlyPicks {
    hour: i32,
    picks: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PickBucket {
    label: String,
    voters: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TopTrackRow {
    id: Uuid,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: f64,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TopVoterRow {
    user_id: Uuid,
    username: String,
    picks: i64,
    duels: i64,
    last_active_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RecentPickRow {
    created_at: DateTime<Utc>,
    username: String,
    winner_title: String,
    winner_artists: String,
    winner_image_url: Option<String>,
    other_title: String,
    other_artists: String,
}

#[derive(Debug, sqlx::FromRow)]
struct InsightsCounts {
    total_tracks: i64,
    tracks_compared: i64,
    previews_missing: i64,
    total_picks: i64,
    picks_today: i64,
    picks_7d: i64,
    total_voters: i64,
    active_voters_24h: i64,
    active_voters_7d: i64,
    total_users: i64,
    admins: i64,
    invites_total: i64,
    invites_used: i64,
    invites_open: i64,
    invites_expired: i64,
    total_duels: i64,
    duel_voters: i64,
    range_picks: i64,
    range_voters: i64,
    range_duels: i64,
    range_duel_voters: i64,
    range_start: String,
    range_end: String,
}

/// A wide slice of mission-control data: totals, activity, coverage, invites,
/// top tracks, top voters and the latest picks.
pub async fn insights(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<InsightsQuery>,
) -> Result<Json<AdminInsights>, ApiError> {
    let _permit = state
        .heavy_work
        .acquire()
        .await
        .map_err(|_| ApiError::service_unavailable("the server is busy; please try again"))?;

    let days = query.days.unwrap_or(14).clamp(1, 90);
    let playlist = active_playlist(&state.db).await?;
    let playlist_id = playlist.as_ref().map(|playlist| playlist.id);

    let counts = sqlx::query_as::<_, InsightsCounts>(
        "SELECT \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1) AS total_tracks, \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1 AND comparisons > 0) AS tracks_compared, \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1 AND preview_url IS NULL) AS previews_missing, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1) AS total_picks, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1 \
             AND created_at >= date_trunc('day', now())) AS picks_today, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1 \
             AND created_at >= now() - interval '7 days') AS picks_7d, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1) AS total_voters, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1 \
             AND created_at >= now() - interval '1 day') AS active_voters_24h, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1 \
             AND created_at >= now() - interval '7 days') AS active_voters_7d, \
            (SELECT COUNT(*) FROM users) AS total_users, \
            (SELECT COUNT(*) FROM users WHERE is_admin) AS admins, \
            (SELECT COUNT(*) FROM invites) AS invites_total, \
            (SELECT COUNT(*) FROM invites WHERE used_at IS NOT NULL) AS invites_used, \
            (SELECT COUNT(*) FROM invites WHERE used_at IS NULL \
             AND (expires_at IS NULL OR expires_at > now())) AS invites_open, \
            (SELECT COUNT(*) FROM invites WHERE used_at IS NULL \
             AND expires_at IS NOT NULL AND expires_at <= now()) AS invites_expired, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1) AS total_duels, \
            (SELECT COUNT(DISTINCT user_id) FROM seen_pairs WHERE playlist_id = $1) AS duel_voters, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1 \
             AND created_at >= current_date - ($2::int - 1) * interval '1 day') AS range_picks, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1 \
             AND created_at >= current_date - ($2::int - 1) * interval '1 day') AS range_voters, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1 \
             AND seen_at >= current_date - ($2::int - 1) * interval '1 day') AS range_duels, \
            (SELECT COUNT(DISTINCT user_id) FROM seen_pairs WHERE playlist_id = $1 \
             AND seen_at >= current_date - ($2::int - 1) * interval '1 day') AS range_duel_voters, \
            to_char(current_date - ($2::int - 1) * interval '1 day', 'YYYY-MM-DD') AS range_start, \
            to_char(current_date, 'YYYY-MM-DD') AS range_end",
    )
    .bind(playlist_id)
    .bind(days)
    .fetch_one(&state.db)
    .await?;

    let daily = sqlx::query_as::<_, InsightDaily>(
        "SELECT to_char(day::date, 'YYYY-MM-DD') AS date, \
                COUNT(v.id) AS votes, \
                COUNT(DISTINCT v.user_id) AS voters \
         FROM generate_series( \
             current_date - ($2::int - 1) * interval '1 day', \
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
    .bind(days)
    .fetch_all(&state.db)
    .await?;

    let hourly = sqlx::query_as::<_, HourlyPicks>(
        "SELECT h.hour, COUNT(v.id) AS picks \
         FROM generate_series(0, 23) AS h(hour) \
         LEFT JOIN votes v \
           ON v.playlist_id = $1 \
          AND v.created_at >= date_trunc('day', now()) \
          AND date_part('hour', v.created_at) = h.hour \
         GROUP BY h.hour \
         ORDER BY h.hour",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let pick_distribution = sqlx::query_as::<_, PickBucket>(
        "SELECT label, COUNT(*) AS voters \
         FROM ( \
             SELECT CASE \
                 WHEN picks = 1 THEN '1 pick' \
                 WHEN picks <= 5 THEN '2–5 picks' \
                 WHEN picks <= 20 THEN '6–20 picks' \
                 WHEN picks <= 50 THEN '21–50 picks' \
                 WHEN picks <= 100 THEN '51–100 picks' \
                 ELSE '100+ picks' \
             END AS label, \
             CASE \
                 WHEN picks = 1 THEN 1 \
                 WHEN picks <= 5 THEN 2 \
                 WHEN picks <= 20 THEN 3 \
                 WHEN picks <= 50 THEN 4 \
                 WHEN picks <= 100 THEN 5 \
                 ELSE 6 \
             END AS sort_order \
             FROM ( \
                 SELECT user_id, COUNT(*) AS picks \
                 FROM votes WHERE playlist_id = $1 \
                 GROUP BY user_id \
             ) counts \
         ) buckets \
         GROUP BY label, sort_order \
         ORDER BY sort_order",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let top_tracks = sqlx::query_as::<_, TopTrackRow>(
        "SELECT id, title, artists, album_image_url, rating, comparisons, wins \
         FROM tracks WHERE playlist_id = $1 \
         ORDER BY rating DESC, wins DESC, title ASC LIMIT 5",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let top_voters = sqlx::query_as::<_, TopVoterRow>(
        "SELECT u.id AS user_id, u.username, \
                COALESCE(v.picks, 0) AS picks, \
                COALESCE(s.duels, 0) AS duels, \
                COALESCE(v.last_pick_at, s.last_seen_at, u.created_at) AS last_active_at \
         FROM users u \
         LEFT JOIN LATERAL ( \
             SELECT COUNT(*) AS picks, MAX(created_at) AS last_pick_at \
             FROM votes \
             WHERE user_id = u.id AND playlist_id = $1 \
         ) v ON TRUE \
         LEFT JOIN LATERAL ( \
             SELECT COUNT(*) AS duels, MAX(seen_at) AS last_seen_at \
             FROM seen_pairs \
             WHERE user_id = u.id AND playlist_id = $1 \
         ) s ON TRUE \
         WHERE COALESCE(v.picks, 0) > 0 OR COALESCE(s.duels, 0) > 0 \
         ORDER BY picks DESC, duels DESC, u.username ASC \
         LIMIT 5",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let recent_picks = sqlx::query_as::<_, RecentPickRow>(
        "SELECT v.created_at, u.username, \
                winner.title AS winner_title, winner.artists AS winner_artists, \
                winner.album_image_url AS winner_image_url, \
                other.title AS other_title, other.artists AS other_artists \
         FROM votes v \
         JOIN users u ON u.id = v.user_id \
         JOIN tracks winner ON winner.id = v.winner \
         JOIN tracks other ON other.id = CASE \
             WHEN v.winner = v.track_low THEN v.track_high ELSE v.track_low \
         END \
         WHERE v.playlist_id = $1 \
         ORDER BY v.created_at DESC, v.id DESC \
         LIMIT 12",
    )
    .bind(playlist_id)
    .fetch_all(&state.db)
    .await?;

    let (playlist_name, playlist_image_url, revealed) = match playlist {
        Some(playlist) => (
            Some(playlist.name),
            playlist.image_url,
            playlist.reveal_state == "revealed",
        ),
        None => (None, None, false),
    };

    let tracks_uncompared = counts.total_tracks - counts.tracks_compared;
    let picks_per_voter = if counts.total_voters > 0 {
        counts.total_picks as f64 / counts.total_voters as f64
    } else {
        0.0
    };
    let duels_per_voter = if counts.duel_voters > 0 {
        counts.total_duels as f64 / counts.duel_voters as f64
    } else {
        0.0
    };
    let coverage_pct = if counts.total_tracks > 0 {
        counts.tracks_compared as f64 / counts.total_tracks as f64 * 100.0
    } else {
        0.0
    };
    let range_picks_per_voter = if counts.range_voters > 0 {
        counts.range_picks as f64 / counts.range_voters as f64
    } else {
        0.0
    };
    let range_duels_per_voter = if counts.range_duel_voters > 0 {
        counts.range_duels as f64 / counts.range_duel_voters as f64
    } else {
        0.0
    };

    tracing::info!(
        playlist_id = ?playlist_id,
        days,
        picks = counts.total_picks,
        range_picks = counts.range_picks,
        voters = counts.total_voters,
        range_voters = counts.range_voters,
        users = counts.total_users,
        duels = counts.total_duels,
        range_duels = counts.range_duels,
        "admin insights computed"
    );

    Ok(Json(AdminInsights {
        playlist_id,
        playlist_name,
        playlist_image_url,
        revealed,
        range_days: days,
        range_start: counts.range_start,
        range_end: counts.range_end,
        range_picks: counts.range_picks,
        range_voters: counts.range_voters,
        range_duels: counts.range_duels,
        range_picks_per_voter,
        range_duels_per_voter,
        total_tracks: counts.total_tracks,
        tracks_compared: counts.tracks_compared,
        tracks_uncompared,
        previews_missing: counts.previews_missing,
        total_picks: counts.total_picks,
        picks_today: counts.picks_today,
        picks_7d: counts.picks_7d,
        picks_per_voter,
        total_voters: counts.total_voters,
        active_voters_24h: counts.active_voters_24h,
        active_voters_7d: counts.active_voters_7d,
        total_users: counts.total_users,
        admins: counts.admins,
        invites_total: counts.invites_total,
        invites_used: counts.invites_used,
        invites_open: counts.invites_open,
        invites_expired: counts.invites_expired,
        total_duels: counts.total_duels,
        duels_per_voter,
        coverage_pct,
        daily,
        hourly,
        pick_distribution,
        top_tracks,
        top_voters,
        recent_picks,
    }))
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

    sqlx::query(
        "UPDATE playlists SET reveal_state = 'hidden', \
                scheduled_reveal_at = NULL, \
                pause_voting_on_reveal = FALSE \
         WHERE id = $1",
    )
    .bind(playlist.id)
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;
    state.cache.clear();

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

pub async fn full_reset(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<FullResetRequest>,
) -> Result<Json<FullResetResponse>, ApiError> {
    if payload.confirm != "RESET EVERYTHING" {
        return Err(ApiError::bad_request(
            "type RESET EVERYTHING to confirm this destructive action",
        ));
    }

    let mut transaction = state.db.begin().await?;

    let deleted_votes = sqlx::query("DELETE FROM votes")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    let deleted_duels = sqlx::query("DELETE FROM seen_pairs")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    sqlx::query("DELETE FROM metadata_backfill_runs")
        .execute(&mut *transaction)
        .await?;

    sqlx::query("DELETE FROM track_artists")
        .execute(&mut *transaction)
        .await?;

    let deleted_tracks = sqlx::query("DELETE FROM tracks")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    let deleted_playlists = sqlx::query("DELETE FROM playlists")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    let deleted_artists = sqlx::query("DELETE FROM artists")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    let deleted_invites = sqlx::query("DELETE FROM invites")
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    // OAuth states are disposable, but the connected Spotify account is
    // infrastructure rather than season data. Keep its stored tokens so a full
    // reset does not force the admin to reconnect before starting the next tape.
    sqlx::query("DELETE FROM spotify_oauth_states")
        .execute(&mut *transaction)
        .await?;

    sqlx::query("DELETE FROM auth_events")
        .execute(&mut *transaction)
        .await?;

    let deleted_users = sqlx::query("DELETE FROM users WHERE id <> $1")
        .bind(admin.0.id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    transaction.commit().await?;
    state.cache.clear();
    state.auth_cache.clear();

    tracing::warn!(
        admin_id = %admin.0.id,
        deleted_users,
        deleted_invites,
        deleted_playlists,
        deleted_tracks,
        deleted_votes,
        deleted_duels,
        deleted_artists,
        "FULL RESET completed"
    );

    Ok(Json(FullResetResponse {
        reset: true,
        deleted_users,
        deleted_invites,
        deleted_playlists,
        deleted_tracks,
        deleted_votes,
        deleted_duels,
        deleted_artists,
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
    state.cache.clear();

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
