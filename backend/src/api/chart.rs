use std::fmt::Write as _;
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::metadata;
use crate::models::{MatchupTrack, Playlist};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ChartStatus {
    revealed: bool,
    playlist_id: Option<Uuid>,
    playlist_name: Option<String>,
    playlist_description: Option<String>,
    playlist_image_url: Option<String>,
    total_tracks: i64,
    tracks_compared: i64,
    votes_cast: i64,
    voters: i64,
    duels: i64,
    total_artists: i64,
    total_albums: i64,
    year_min: Option<i32>,
    year_max: Option<i32>,
    scheduled_start_at: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    voting_paused: bool,
    voting_paused_at: Option<DateTime<Utc>>,
    scheduled_reveal_at: Option<DateTime<Utc>>,
    pause_voting_on_reveal: bool,
    decades: Vec<FacetCount>,
    genres: Vec<FacetCount>,
    artists: Vec<ArtistFacet>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FacetCount {
    value: String,
    count: i64,
    average_rating: Option<f64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ArtistFacet {
    id: String,
    name: String,
    count: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChartQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
    pub decade: Option<i32>,
    pub genre: Option<String>,
    pub artist: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PublicChart {
    playlist_id: Uuid,
    playlist_name: String,
    revealed: bool,
    total: i64,
    limit: i64,
    offset: i64,
    query: String,
    decade: Option<i32>,
    genre: Option<String>,
    tracks: Vec<ChartEntry>,
}

#[derive(Debug, Serialize)]
pub struct ChartEntry {
    rank: usize,
    track: MatchupTrack,
    rating: f64,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, Deserialize)]
pub struct RevealRequest {
    revealed: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct ChartRow {
    id: Uuid,
    spotify_id: String,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    preview_url: Option<String>,
    duration_ms: Option<i32>,
    release_year: Option<i32>,
    decade: Option<i32>,
    album_id: Option<String>,
    album_type: Option<String>,
    total_tracks: Option<i32>,
    disc_number: Option<i32>,
    track_number: Option<i32>,
    primary_genre: Option<String>,
    rating: f64,
    comparisons: i32,
    wins: i32,
    official_rank: i64,
}

impl ChartRow {
    fn into_entry(self) -> ChartEntry {
        ChartEntry {
            rank: self.official_rank as usize,
            track: MatchupTrack {
                id: self.id,
                spotify_id: self.spotify_id,
                title: self.title,
                artists: self.artists,
                album: self.album,
                album_image_url: self.album_image_url,
                preview_url: self.preview_url,
                duration_ms: self.duration_ms,
                release_year: self.release_year,
                decade: self.decade,
                album_id: self.album_id,
                album_type: self.album_type,
                total_tracks: self.total_tracks,
                disc_number: self.disc_number,
                track_number: self.track_number,
                primary_genre: self.primary_genre,
            },
            rating: self.rating,
            comparisons: self.comparisons,
            wins: self.wins,
        }
    }
}

fn empty_status() -> ChartStatus {
    ChartStatus {
        revealed: false,
        playlist_id: None,
        playlist_name: None,
        playlist_description: None,
        playlist_image_url: None,
        total_tracks: 0,
        tracks_compared: 0,
        votes_cast: 0,
        voters: 0,
        duels: 0,
        total_artists: 0,
        total_albums: 0,
        year_min: None,
        year_max: None,
        scheduled_start_at: None,
        started_at: None,
        voting_paused: false,
        voting_paused_at: None,
        scheduled_reveal_at: None,
        pause_voting_on_reveal: false,
        decades: Vec::new(),
        genres: Vec::new(),
        artists: Vec::new(),
    }
}

/// Public metadata about the chart. Safe to expose while it is still sealed.
pub async fn status(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let value = state
        .cache
        .get_or_compute(
            "chart:status".to_string(),
            Duration::from_secs(10),
            || async {
                let _permit = state.heavy_work.acquire().await.map_err(|_| {
                    ApiError::service_unavailable("the server is busy; please try again")
                })?;

                let Some(playlist) = active_playlist(&state.db).await? else {
                    return serde_json::to_value(empty_status()).map_err(ApiError::internal);
                };

                let status = build_status(&state.db, &playlist).await?;
                serde_json::to_value(status).map_err(ApiError::internal)
            },
        )
        .await?;

    Ok(Json(value))
}

/// The shared Elo chart. Only available after the admin reveal.
pub async fn public(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<Value>, ApiError> {
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

    let key = format!(
        "chart:public:{}:{limit}:{offset}:{}:{:?}:{:?}:{:?}",
        playlist.id, search, query.decade, genre, query.artist
    );

    let value = state
        .cache
        .get_or_compute(key, Duration::from_secs(10), || async {
            let _permit = state.heavy_work.acquire().await.map_err(|_| {
                ApiError::service_unavailable("the server is busy; please try again")
            })?;

            let chart = load_chart(
                &state.db,
                &playlist,
                limit,
                offset,
                &search,
                query.decade,
                genre.as_deref(),
                query.artist,
            )
            .await?;
            serde_json::to_value(chart).map_err(ApiError::internal)
        })
        .await?;

    Ok(Json(value))
}

// The filter arguments are kept explicit because this loader is shared by the
// public chart and the admin ranking; a struct would add noise without making
// the SQL construction clearer.
#[allow(clippy::too_many_arguments)]
pub async fn load_chart(
    pool: &PgPool,
    playlist: &Playlist,
    limit: i64,
    offset: i64,
    search: &str,
    decade: Option<i32>,
    genre: Option<&str>,
    artist: Option<Uuid>,
) -> Result<PublicChart, ApiError> {
    crate::error::validate_text_input(search, "search")?;
    if let Some(genre) = genre {
        crate::error::validate_text_input(genre, "genre")?;
    }

    const FILTER: &str = "t.playlist_id = $1 \
         AND ($2 = '' OR t.title ILIKE '%' || $2 || '%' \
              OR t.artists ILIKE '%' || $2 || '%' \
              OR t.album ILIKE '%' || $2 || '%') \
         AND ($3::int IS NULL OR t.decade = $3) \
         AND ($4::text IS NULL OR t.primary_genre = $4) \
         AND ($5::uuid IS NULL OR EXISTS ( \
             SELECT 1 FROM track_artists ta \
             WHERE ta.track_id = t.id AND ta.artist_id = $5 \
         ))";

    // Rank the whole playlist first, then filter. This keeps the official rank
    // visible when a voter searches or narrows the standings by facet.
    const RANKED: &str = "WITH ranked AS ( \
         SELECT t.*, ROW_NUMBER() OVER ( \
             ORDER BY t.rating DESC, t.wins DESC, t.title ASC, t.id ASC \
         ) AS official_rank \
         FROM tracks t \
         WHERE t.playlist_id = $1 \
     ) ";

    let total = sqlx::query_scalar::<_, i64>(&format!(
        "{RANKED} SELECT COUNT(*) FROM ranked t WHERE {FILTER}"
    ))
    .bind(playlist.id)
    .bind(search)
    .bind(decade)
    .bind(genre)
    .bind(artist)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, ChartRow>(&format!(
        "{RANKED} \
         SELECT t.id, t.spotify_id, t.title, t.artists, t.album, t.album_image_url, \
                t.preview_url, t.duration_ms, t.release_year, t.decade, t.album_id, \
                t.album_type, t.total_tracks, t.disc_number, t.track_number, \
                t.primary_genre, t.rating, t.comparisons, t.wins, t.official_rank \
         FROM ranked t \
         WHERE {FILTER} \
         ORDER BY t.official_rank ASC \
         LIMIT $6 OFFSET $7"
    ))
    .bind(playlist.id)
    .bind(search)
    .bind(decade)
    .bind(genre)
    .bind(artist)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let tracks = rows.into_iter().map(ChartRow::into_entry).collect();

    Ok(PublicChart {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: true,
        total,
        limit,
        offset,
        query: search.to_string(),
        decade,
        genre: genre.map(str::to_string),
        tracks,
    })
}

pub async fn reveal(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<RevealRequest>,
) -> Result<Json<ChartStatus>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let reveal_state = if payload.revealed {
        "revealed"
    } else {
        "hidden"
    };

    sqlx::query(
        "UPDATE playlists SET reveal_state = $1, \
                scheduled_reveal_at = NULL, pause_voting_on_reveal = FALSE \
         WHERE id = $2",
    )
    .bind(reveal_state)
    .bind(playlist.id)
    .execute(&state.db)
    .await?;

    state.cache.clear();

    let updated = Playlist {
        reveal_state: reveal_state.to_string(),
        scheduled_reveal_at: None,
        pause_voting_on_reveal: false,
        ..playlist
    };
    let status = build_status(&state.db, &updated).await?;

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %updated.id,
        revealed = payload.revealed,
        "standings reveal state changed"
    );

    Ok(Json(status))
}

#[derive(Debug, Deserialize)]
pub struct RevealScheduleRequest {
    reveal_at: DateTime<Utc>,
    pause_voting: bool,
}

pub async fn schedule_reveal(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<RevealScheduleRequest>,
) -> Result<Json<ChartStatus>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no playlist has been set yet".to_string(),
        ));
    };

    if playlist.reveal_state == "revealed" {
        return Err(ApiError::Conflict(
            "the standings are already revealed".to_string(),
        ));
    }

    if payload.reveal_at <= Utc::now() {
        return Err(ApiError::bad_request(
            "the scheduled reveal must be in the future",
        ));
    }

    if playlist.started_at.is_none()
        && playlist
            .scheduled_start_at
            .is_some_and(|start| payload.reveal_at < start)
    {
        return Err(ApiError::bad_request(
            "the reveal must be after the scheduled start",
        ));
    }

    sqlx::query(
        "UPDATE playlists SET scheduled_reveal_at = $2, pause_voting_on_reveal = $3 \
         WHERE id = $1",
    )
    .bind(playlist.id)
    .bind(payload.reveal_at)
    .bind(payload.pause_voting)
    .execute(&state.db)
    .await?;

    state.cache.clear();

    let updated = Playlist {
        scheduled_reveal_at: Some(payload.reveal_at),
        pause_voting_on_reveal: payload.pause_voting,
        ..playlist
    };

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %updated.id,
        reveal_at = %payload.reveal_at,
        pause_voting = payload.pause_voting,
        "scheduled reveal set"
    );

    Ok(Json(build_status(&state.db, &updated).await?))
}

pub async fn cancel_scheduled_reveal(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<ChartStatus>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no playlist has been set yet".to_string(),
        ));
    };

    sqlx::query(
        "UPDATE playlists SET scheduled_reveal_at = NULL, \
                pause_voting_on_reveal = FALSE \
         WHERE id = $1",
    )
    .bind(playlist.id)
    .execute(&state.db)
    .await?;

    state.cache.clear();

    let updated = Playlist {
        scheduled_reveal_at: None,
        pause_voting_on_reveal: false,
        ..playlist
    };

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %updated.id,
        "scheduled reveal cancelled"
    );

    Ok(Json(build_status(&state.db, &updated).await?))
}

pub async fn export_csv(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Response, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let rows = sqlx::query_as::<_, ChartRow>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, release_year, decade, album_id, album_type, total_tracks, \
                disc_number, track_number, primary_genre, rating, comparisons, wins, \
                ROW_NUMBER() OVER ( \
                    ORDER BY rating DESC, wins DESC, title ASC, id ASC \
                ) AS official_rank \
         FROM tracks WHERE playlist_id = $1 \
         ORDER BY rating DESC, wins DESC, title ASC, id ASC",
    )
    .bind(playlist.id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from("rank,title,artists,album,spotify_id,rating,comparisons,wins\n");

    for row in rows.iter() {
        let _ = writeln!(
            &mut csv,
            "{},{},{},{},{},{:.2},{},{}",
            row.official_rank,
            csv_field(&row.title),
            csv_field(&row.artists),
            csv_field(row.album.as_deref().unwrap_or("")),
            csv_field(&row.spotify_id),
            row.rating,
            row.comparisons,
            row.wins
        );
    }

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.id,
        rows = rows.len(),
        "standings CSV exported"
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"playlist-voter.csv\"",
        )
        .body(Body::from(csv))
        .map_err(ApiError::internal)
}

async fn build_status(pool: &PgPool, playlist: &Playlist) -> Result<ChartStatus, ApiError> {
    #[derive(Debug, sqlx::FromRow)]
    struct StatusCounts {
        total_tracks: i64,
        tracks_compared: i64,
        votes_cast: i64,
        voters: i64,
        duels: i64,
        total_artists: i64,
        total_albums: i64,
        year_min: Option<i32>,
        year_max: Option<i32>,
    }

    let counts = sqlx::query_as::<_, StatusCounts>(
        "SELECT \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1) AS total_tracks, \
            (SELECT COUNT(*) FROM tracks WHERE playlist_id = $1 AND comparisons > 0) AS tracks_compared, \
            (SELECT COUNT(*) FROM votes WHERE playlist_id = $1) AS votes_cast, \
            (SELECT COUNT(DISTINCT user_id) FROM votes WHERE playlist_id = $1) AS voters, \
            (SELECT COUNT(*) FROM seen_pairs WHERE playlist_id = $1) AS duels, \
            (SELECT COUNT(DISTINCT ta.artist_id) FROM track_artists ta \
             JOIN tracks t ON t.id = ta.track_id WHERE t.playlist_id = $1) AS total_artists, \
            (SELECT COUNT(DISTINCT album_id) FROM tracks \
             WHERE playlist_id = $1 AND album_id IS NOT NULL) AS total_albums, \
            (SELECT MIN(release_year) FROM tracks WHERE playlist_id = $1) AS year_min, \
            (SELECT MAX(release_year) FROM tracks WHERE playlist_id = $1) AS year_max",
    )
    .bind(playlist.id)
    .fetch_one(pool)
    .await?;

    let mut decades = sqlx::query_as::<_, FacetCount>(
        "SELECT decade::text AS value, COUNT(*) AS count, \
                AVG(rating) FILTER (WHERE comparisons > 0)::float8 AS average_rating \
         FROM tracks \
         WHERE playlist_id = $1 AND decade IS NOT NULL \
         GROUP BY decade ORDER BY decade ASC",
    )
    .bind(playlist.id)
    .fetch_all(pool)
    .await?;

    let mut genres = sqlx::query_as::<_, FacetCount>(
        "SELECT primary_genre AS value, COUNT(*) AS count, \
                AVG(rating) FILTER (WHERE comparisons > 0)::float8 AS average_rating \
         FROM tracks \
         WHERE playlist_id = $1 AND primary_genre IS NOT NULL \
         GROUP BY primary_genre ORDER BY count DESC, value ASC",
    )
    .bind(playlist.id)
    .fetch_all(pool)
    .await?;

    let artists = sqlx::query_as::<_, ArtistFacet>(
        "SELECT a.id::text AS id, a.name, COUNT(*)::bigint AS count \
         FROM track_artists ta \
         JOIN artists a ON a.id = ta.artist_id \
         JOIN tracks t ON t.id = ta.track_id \
         WHERE t.playlist_id = $1 \
         GROUP BY a.id, a.name \
         ORDER BY a.name ASC",
    )
    .bind(playlist.id)
    .fetch_all(pool)
    .await?;

    // The status endpoint is public even while the standings are sealed. Facet
    // counts are safe to expose, but average ratings would leak score
    // information before The Reveal.
    if playlist.reveal_state != "revealed" {
        for facet in decades.iter_mut().chain(genres.iter_mut()) {
            facet.average_rating = None;
        }
    }

    // Keep the filter chips in the same stable order as the broad buckets.
    genres.sort_by_key(|facet| {
        metadata::BROAD_GENRES
            .iter()
            .position(|genre| *genre == facet.value)
            .unwrap_or(usize::MAX)
    });

    Ok(ChartStatus {
        revealed: playlist.reveal_state == "revealed",
        playlist_id: Some(playlist.id),
        playlist_name: Some(playlist.name.clone()),
        playlist_description: playlist.description.clone(),
        playlist_image_url: playlist.image_url.clone(),
        total_tracks: counts.total_tracks,
        tracks_compared: counts.tracks_compared,
        votes_cast: counts.votes_cast,
        voters: counts.voters,
        duels: counts.duels,
        total_artists: counts.total_artists,
        total_albums: counts.total_albums,
        year_min: counts.year_min,
        year_max: counts.year_max,
        scheduled_start_at: playlist.scheduled_start_at,
        started_at: playlist.started_at,
        voting_paused: playlist.voting_paused,
        voting_paused_at: playlist.voting_paused_at,
        scheduled_reveal_at: playlist.scheduled_reveal_at,
        pause_voting_on_reveal: playlist.pause_voting_on_reveal,
        decades,
        genres,
        artists,
    })
}

pub(crate) fn csv_field(value: &str) -> String {
    // Spreadsheet formula injection: quoting is not enough. Neutralise values
    // whose first non-whitespace character could start a formula, then apply
    // normal RFC-4180 quoting.
    let first = value
        .trim_start_matches([' ', '\t', '\r', '\n'])
        .chars()
        .next();
    let dangerous = matches!(first, Some('=' | '+' | '-' | '@' | '\t' | '\r'));
    let value = if dangerous {
        format!("'{value}")
    } else {
        value.to_string()
    };

    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_status_is_sealed_and_zeroed() {
        let status = empty_status();
        assert!(!status.revealed);
        assert_eq!(status.playlist_id, None);
        assert_eq!(status.total_tracks, 0);
        assert_eq!(status.votes_cast, 0);
        assert!(status.decades.is_empty());
        assert!(status.genres.is_empty());
        assert!(status.artists.is_empty());
    }

    #[test]
    fn csv_fields_leave_safe_values_unchanged() {
        assert_eq!(csv_field(""), "");
        assert_eq!(csv_field("Song Title"), "Song Title");
        assert_eq!(csv_field("Björk"), "Björk");
    }

    #[test]
    fn csv_fields_quote_carriage_returns_and_commas() {
        assert_eq!(csv_field("a\rb"), "\"a\rb\"");
        assert_eq!(csv_field("a,b,c"), "\"a,b,c\"");
    }

    #[test]
    fn csv_fields_escape_embedded_quotes() {
        assert_eq!(csv_field("\"quoted\""), "\"\"\"quoted\"\"\"");
    }

    #[test]
    fn csv_fields_neutralise_every_formula_prefix() {
        for value in ["=1+1", "+1", "-1", "@cmd", "\t=1"] {
            let escaped = csv_field(value);
            assert!(
                escaped.starts_with('\'') || escaped.starts_with("\"'"),
                "{value:?} -> {escaped:?}"
            );
        }
    }

    #[test]
    fn csv_fields_neutralise_formulas_hidden_behind_spaces() {
        let escaped = csv_field("   =SUM(A1)");
        assert_eq!(escaped, "'   =SUM(A1)");
    }

    #[test]
    fn csv_fields_do_not_treat_mid_string_equals_as_a_formula() {
        assert_eq!(csv_field("a=b"), "a=b");
    }

    #[test]
    fn csv_fields_do_not_require_quoting_for_a_quote_inside_a_safe_value() {
        assert_eq!(csv_field("rock 'n' roll"), "rock 'n' roll");
    }

    #[test]
    fn csv_fields_are_escaped() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("with, comma"), "\"with, comma\"");
        assert_eq!(csv_field("with \"quote\""), "\"with \"\"quote\"\"\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
        assert_eq!(csv_field("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
        assert_eq!(csv_field("+1"), "'+1");
        assert_eq!(csv_field("@cmd"), "'@cmd");
        assert_eq!(csv_field("  -2"), "'  -2");
    }
}
