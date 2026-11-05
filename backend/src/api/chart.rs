use std::fmt::Write as _;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::{MatchupTrack, Playlist};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ChartStatus {
    revealed: bool,
    playlist_id: Option<Uuid>,
    playlist_name: Option<String>,
    total_tracks: i64,
    votes_cast: i64,
}

#[derive(Debug, Deserialize)]
pub struct ChartQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub q: Option<String>,
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
    rating: f64,
    comparisons: i32,
    wins: i32,
}

impl ChartRow {
    fn into_entry(self, rank: usize) -> ChartEntry {
        ChartEntry {
            rank,
            track: MatchupTrack {
                id: self.id,
                spotify_id: self.spotify_id,
                title: self.title,
                artists: self.artists,
                album: self.album,
                album_image_url: self.album_image_url,
                preview_url: self.preview_url,
                duration_ms: self.duration_ms,
            },
            rating: self.rating,
            comparisons: self.comparisons,
            wins: self.wins,
        }
    }
}

/// Public metadata about the chart. Safe to expose while it is still sealed.
pub async fn status(State(state): State<AppState>) -> Result<Json<ChartStatus>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Ok(Json(ChartStatus {
            revealed: false,
            playlist_id: None,
            playlist_name: None,
            total_tracks: 0,
            votes_cast: 0,
        }));
    };

    Ok(Json(build_status(&state.db, &playlist).await?))
}

/// The shared Elo chart. Only available after the admin reveal.
pub async fn public(
    State(state): State<AppState>,
    Query(query): Query<ChartQuery>,
) -> Result<Json<PublicChart>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    if playlist.reveal_state != "revealed" {
        return Err(ApiError::Forbidden(
            "the shared chart has not been revealed yet".to_string(),
        ));
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();

    Ok(Json(
        load_chart(&state.db, &playlist, limit, offset, &search).await?,
    ))
}

pub async fn load_chart(
    pool: &PgPool,
    playlist: &Playlist,
    limit: i64,
    offset: i64,
    search: &str,
) -> Result<PublicChart, ApiError> {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tracks \
         WHERE playlist_id = $1 \
           AND ($2 = '' OR title ILIKE '%' || $2 || '%' \
                OR artists ILIKE '%' || $2 || '%' \
                OR album ILIKE '%' || $2 || '%')",
    )
    .bind(playlist.id)
    .bind(search)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, ChartRow>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, rating, comparisons, wins \
         FROM tracks \
         WHERE playlist_id = $1 \
           AND ($2 = '' OR title ILIKE '%' || $2 || '%' \
                OR artists ILIKE '%' || $2 || '%' \
                OR album ILIKE '%' || $2 || '%') \
         ORDER BY rating DESC, wins DESC, title ASC, id ASC \
         LIMIT $3 OFFSET $4",
    )
    .bind(playlist.id)
    .bind(search)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let start = usize::try_from(offset).unwrap_or(0);
    let tracks = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| row.into_entry(start + index + 1))
        .collect();

    Ok(PublicChart {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: true,
        total,
        limit,
        offset,
        query: search.to_string(),
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

    sqlx::query("UPDATE playlists SET reveal_state = $1 WHERE id = $2")
        .bind(reveal_state)
        .bind(playlist.id)
        .execute(&state.db)
        .await?;

    let updated = Playlist {
        reveal_state: reveal_state.to_string(),
        ..playlist
    };
    let status = build_status(&state.db, &updated).await?;

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %updated.id,
        revealed = payload.revealed,
        "shared chart reveal state changed"
    );

    Ok(Json(status))
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
                duration_ms, rating, comparisons, wins \
         FROM tracks WHERE playlist_id = $1 \
         ORDER BY rating DESC, wins DESC, title ASC, id ASC",
    )
    .bind(playlist.id)
    .fetch_all(&state.db)
    .await?;

    let mut csv = String::from(
        "rank,title,artists,album,spotify_id,rating,comparisons,wins\n",
    );

    for (index, row) in rows.iter().enumerate() {
        let _ = writeln!(
            &mut csv,
            "{},{},{},{},{},{:.2},{},{}",
            index + 1,
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
        "chart CSV exported"
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"ull-top-2000.csv\"",
        )
        .body(Body::from(csv))
        .map_err(|error| ApiError::internal(error))
}

async fn build_status(pool: &PgPool, playlist: &Playlist) -> Result<ChartStatus, ApiError> {
    let total_tracks =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(pool)
            .await?;

    let votes_cast =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM votes WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(pool)
            .await?;

    Ok(ChartStatus {
        revealed: playlist.reveal_state == "revealed",
        playlist_id: Some(playlist.id),
        playlist_name: Some(playlist.name.clone()),
        total_tracks,
        votes_cast,
    })
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_fields_are_escaped() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("with, comma"), "\"with, comma\"");
        assert_eq!(csv_field("with \"quote\""), "\"with \"\"quote\"\"\"");
        assert_eq!(csv_field("line\nbreak"), "\"line\nbreak\"");
    }
}
