//! Artist and album views derived from the official track ranking.
//!
//! These views never change the ranking; they only group the same Elo scores
//! by artist or release. Like the standings, they stay sealed until The Reveal.

use std::time::Duration;

use axum::extract::{Query, State};
use axum::Json;
use serde::Serialize;
use serde_json::Value;
use sqlx::PgPool;

use crate::api::chart::ChartQuery;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::Playlist;
use crate::state::AppState;

const FILTER: &str = "t.playlist_id = $1 \
     AND ($2 = '' OR t.title ILIKE '%' || $2 || '%' \
          OR t.artists ILIKE '%' || $2 || '%' \
          OR t.album ILIKE '%' || $2 || '%') \
     AND ($3::int IS NULL OR t.decade = $3) \
     AND ($4::text IS NULL OR t.primary_genre = $4) \
     AND ($5::uuid IS NULL OR EXISTS ( \
         SELECT 1 FROM track_artists ta2 \
         WHERE ta2.track_id = t.id AND ta2.artist_id = $5 \
     )) \
     AND t.comparisons > 0";

#[derive(Debug, Serialize)]
pub struct AggregateResponse {
    playlist_id: uuid::Uuid,
    playlist_name: String,
    revealed: bool,
    view: &'static str,
    total: i64,
    limit: i64,
    offset: i64,
    query: String,
    decade: Option<i32>,
    genre: Option<String>,
    entries: Vec<AggregateEntry>,
}

#[derive(Debug, Serialize)]
pub struct AggregateEntry {
    id: String,
    name: String,
    subtitle: Option<String>,
    image_url: Option<String>,
    genre: Option<String>,
    album_type: Option<String>,
    release_year: Option<i32>,
    decade: Option<i32>,
    track_count: i64,
    rating: f64,
    comparisons: i64,
    wins: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct ArtistRow {
    id: String,
    name: String,
    genre: Option<String>,
    image_url: Option<String>,
    track_count: i64,
    rating: f64,
    comparisons: i64,
    wins: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AlbumRow {
    id: String,
    name: String,
    artists: Option<String>,
    image_url: Option<String>,
    album_type: Option<String>,
    release_year: Option<i32>,
    decade: Option<i32>,
    track_count: i64,
    rating: f64,
    comparisons: i64,
    wins: i64,
}

fn parsed_filters(query: &ChartQuery) -> (String, Option<i32>, Option<String>, Option<uuid::Uuid>) {
    let search = query.q.clone().unwrap_or_default().trim().to_string();
    let genre = query
        .genre
        .clone()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    (search, query.decade, genre, query.artist)
}

pub async fn artists(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<Value>, ApiError> {
    let playlist = revealed_playlist(&state).await?;
    let (search, decade, genre, artist) = parsed_filters(&query);
    crate::error::validate_text_input(&search, "search")?;
    if let Some(genre) = genre.as_deref() {
        crate::error::validate_text_input(genre, "genre")?;
    }
    let (limit, offset) = paging(&query);
    let key = format!(
        "chart:artists:{}:{limit}:{offset}:{}:{:?}:{:?}:{:?}",
        playlist.id, search, decade, genre, artist
    );

    let value = state
        .cache
        .get_or_compute(key, Duration::from_secs(10), || async {
            let _permit = state.heavy_work.acquire().await.map_err(|_| {
                ApiError::service_unavailable("the server is busy; please try again")
            })?;

            let response = load_artists(
                &state.db,
                &playlist,
                &search,
                decade,
                genre.as_deref(),
                artist,
                &query,
            )
            .await?;
            serde_json::to_value(response).map_err(ApiError::internal)
        })
        .await?;

    Ok(Json(value))
}

pub async fn albums(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<Value>, ApiError> {
    let playlist = revealed_playlist(&state).await?;
    let (search, decade, genre, artist) = parsed_filters(&query);
    crate::error::validate_text_input(&search, "search")?;
    if let Some(genre) = genre.as_deref() {
        crate::error::validate_text_input(genre, "genre")?;
    }
    let (limit, offset) = paging(&query);
    let key = format!(
        "chart:albums:{}:{limit}:{offset}:{}:{:?}:{:?}:{:?}",
        playlist.id, search, decade, genre, artist
    );

    let value = state
        .cache
        .get_or_compute(key, Duration::from_secs(10), || async {
            let _permit = state.heavy_work.acquire().await.map_err(|_| {
                ApiError::service_unavailable("the server is busy; please try again")
            })?;

            let response = load_albums(
                &state.db,
                &playlist,
                &search,
                decade,
                genre.as_deref(),
                artist,
                &query,
            )
            .await?;
            serde_json::to_value(response).map_err(ApiError::internal)
        })
        .await?;

    Ok(Json(value))
}

async fn revealed_playlist(state: &AppState) -> Result<Playlist, ApiError> {
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

    Ok(playlist)
}

fn paging(query: &ChartQuery) -> (i64, i64) {
    (
        query.limit.unwrap_or(50).clamp(1, 200),
        query.offset.unwrap_or(0).max(0),
    )
}

pub async fn load_artists(
    pool: &PgPool,
    playlist: &Playlist,
    search: &str,
    decade: Option<i32>,
    genre: Option<&str>,
    artist: Option<uuid::Uuid>,
    query: &ChartQuery,
) -> Result<AggregateResponse, ApiError> {
    let (limit, offset) = paging(query);

    let total = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM ( \
            SELECT a.id FROM track_artists ta \
            JOIN artists a ON a.id = ta.artist_id \
            JOIN tracks t ON t.id = ta.track_id \
            WHERE {FILTER} \
            GROUP BY a.id \
         ) grouped"
    ))
    .bind(playlist.id)
    .bind(search)
    .bind(decade)
    .bind(genre)
    .bind(artist)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, ArtistRow>(&format!(
        "SELECT a.id::text AS id, a.name, a.primary_genre AS genre, \
                (array_agg(t.album_image_url ORDER BY t.rating DESC) \
                    FILTER (WHERE t.album_image_url IS NOT NULL))[1] AS image_url, \
                COUNT(*)::bigint AS track_count, \
                AVG(t.rating)::float8 AS rating, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins \
         FROM track_artists ta \
         JOIN artists a ON a.id = ta.artist_id \
         JOIN tracks t ON t.id = ta.track_id \
         WHERE {FILTER} \
         GROUP BY a.id, a.name, a.primary_genre \
         ORDER BY rating DESC, wins DESC, track_count DESC, a.name ASC \
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

    let entries = rows
        .into_iter()
        .map(|row| AggregateEntry {
            id: row.id,
            name: row.name,
            subtitle: row.genre.clone(),
            image_url: row.image_url,
            genre: row.genre,
            album_type: None,
            release_year: None,
            decade: None,
            track_count: row.track_count,
            rating: row.rating,
            comparisons: row.comparisons,
            wins: row.wins,
        })
        .collect();

    Ok(AggregateResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: true,
        view: "artists",
        total,
        limit,
        offset,
        query: search.to_string(),
        decade,
        genre: genre.map(str::to_string),
        entries,
    })
}

pub async fn load_albums(
    pool: &PgPool,
    playlist: &Playlist,
    search: &str,
    decade: Option<i32>,
    genre: Option<&str>,
    artist: Option<uuid::Uuid>,
    query: &ChartQuery,
) -> Result<AggregateResponse, ApiError> {
    let (limit, offset) = paging(query);

    let total = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM ( \
            SELECT t.album_id FROM tracks t \
            WHERE {FILTER} AND t.album_id IS NOT NULL \
            GROUP BY t.album_id \
         ) grouped"
    ))
    .bind(playlist.id)
    .bind(search)
    .bind(decade)
    .bind(genre)
    .bind(artist)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, AlbumRow>(&format!(
        "SELECT t.album_id AS id, COALESCE(MAX(t.album), t.album_id) AS name, \
                MAX(t.artists) AS artists, MAX(t.album_image_url) AS image_url, \
                MAX(t.album_type) AS album_type, MAX(t.release_year) AS release_year, \
                MAX(t.decade) AS decade, COUNT(*)::bigint AS track_count, \
                AVG(t.rating)::float8 AS rating, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins \
         FROM tracks t \
         WHERE {FILTER} AND t.album_id IS NOT NULL \
         GROUP BY t.album_id \
         ORDER BY rating DESC, wins DESC, track_count DESC, name ASC \
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

    let entries = rows
        .into_iter()
        .map(|row| AggregateEntry {
            id: row.id,
            name: row.name,
            subtitle: row.artists,
            image_url: row.image_url,
            genre: None,
            album_type: row.album_type,
            release_year: row.release_year,
            decade: row.decade,
            track_count: row.track_count,
            rating: row.rating,
            comparisons: row.comparisons,
            wins: row.wins,
        })
        .collect();

    Ok(AggregateResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: true,
        view: "albums",
        total,
        limit,
        offset,
        query: search.to_string(),
        decade,
        genre: genre.map(str::to_string),
        entries,
    })
}
