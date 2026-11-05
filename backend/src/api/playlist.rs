use std::time::Instant;

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::api::spotify::map_error;
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::{Playlist, Track};
use crate::spotify::{self, ImportedPlaylist};
use crate::spotify_auth;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    playlist: String,
}

#[derive(Debug, Serialize)]
pub struct ImportSummary {
    playlist: Playlist,
    imported_tracks: usize,
    skipped_tracks: usize,
    total_tracks: i64,
    previews_missing: i64,
}

#[derive(Debug, Serialize)]
pub struct PlaylistOverview {
    playlist: Option<Playlist>,
    track_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct TracksQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TracksPage {
    total: i64,
    limit: i64,
    offset: i64,
    tracks: Vec<Track>,
}

pub async fn import(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<ImportRequest>,
) -> Result<Json<ImportSummary>, ApiError> {
    let playlist_id = spotify::parse_playlist_id(&payload.playlist).ok_or_else(|| {
        ApiError::bad_request(
            "expected a Spotify playlist URL, URI, or 22-character playlist ID",
        )
    })?;

    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify import is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;

    let started = Instant::now();
    tracing::info!(
        admin_id = %admin.0.id,
        spotify_playlist_id = %playlist_id,
        "playlist import started"
    );

    let user_token = spotify_auth::user_access_token(client, &state.db)
        .await
        .map_err(|error| map_error(error, true))?;
    let connected = user_token.is_some();
    let auth_mode = if connected {
        "oauth"
    } else {
        "client_credentials"
    };

    let imported = match user_token {
        Some(token) => {
            client
                .fetch_playlist_with_token(&playlist_id, &token)
                .await
        }
        None => client.fetch_playlist(&playlist_id).await,
    }
    .map_err(|error| map_error(error, connected))?;

    tracing::info!(
        admin_id = %admin.0.id,
        spotify_playlist_id = %imported.spotify_id,
        name = %imported.name,
        tracks = imported.tracks.len(),
        skipped = imported.skipped_tracks,
        auth_mode,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "Spotify playlist fetched"
    );

    let (playlist, total_tracks) =
        persist_import(&state.db, admin.0.id, &imported).await?;

    tracing::info!(
        playlist_id = %playlist.id,
        total_tracks,
        "playlist import saved"
    );

    let previews_missing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tracks \
         WHERE playlist_id = $1 AND preview_url IS NULL",
    )
    .bind(playlist.id)
    .fetch_one(&state.db)
    .await?;

    if previews_missing > 0 {
        crate::preview_backfill::spawn(
            client.clone(),
            state.db.clone(),
            playlist.id,
        );
        tracing::info!(
            playlist_id = %playlist.id,
            missing = previews_missing,
            "started preview backfill after import"
        );
    }

    Ok(Json(ImportSummary {
        playlist,
        imported_tracks: imported.tracks.len(),
        skipped_tracks: imported.skipped_tracks,
        total_tracks,
        previews_missing,
    }))
}

#[derive(Debug, Serialize)]
pub struct BackfillPreviewsResponse {
    started: bool,
    missing: i64,
}

pub async fn backfill_previews(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<BackfillPreviewsResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;

    let missing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tracks \
         WHERE playlist_id = $1 AND preview_url IS NULL",
    )
    .bind(playlist.id)
    .fetch_one(&state.db)
    .await?;

    if missing > 0 {
        crate::preview_backfill::spawn(
            client.clone(),
            state.db.clone(),
            playlist.id,
        );
        tracing::info!(
            playlist_id = %playlist.id,
            missing,
            "manual preview backfill started"
        );
    }

    Ok(Json(BackfillPreviewsResponse {
        started: missing > 0,
        missing,
    }))
}

pub async fn overview(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<PlaylistOverview>, ApiError> {
    let playlist = active_playlist(&state.db).await?;

    let track_count = match &playlist {
        Some(playlist) => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
                .bind(playlist.id)
                .fetch_one(&state.db)
                .await?
        }
        None => 0,
    };

    Ok(Json(PlaylistOverview {
        playlist,
        track_count,
    }))
}

pub async fn tracks(
    State(state): State<AppState>,
    _admin: AdminUser,
    Query(query): Query<TracksQuery>,
) -> Result<Json<TracksPage>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no playlist has been imported yet".to_string(),
        ));
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let total =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&state.db)
            .await?;

    let tracks = sqlx::query_as::<_, Track>(
        "SELECT id, playlist_id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, position, rating, comparisons, wins \
         FROM tracks WHERE playlist_id = $1 \
         ORDER BY position ASC, id ASC LIMIT $2 OFFSET $3",
    )
    .bind(playlist.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(TracksPage {
        total,
        limit,
        offset,
        tracks,
    }))
}

async fn persist_import(
    pool: &PgPool,
    admin_id: Uuid,
    imported: &ImportedPlaylist,
) -> Result<(Playlist, i64), ApiError> {
    let mut transaction = pool.begin().await?;

    // Only one playlist can be active at a time.
    sqlx::query(
        "UPDATE playlists SET is_active = FALSE \
         WHERE is_active = TRUE AND spotify_id <> $1",
    )
    .bind(&imported.spotify_id)
    .execute(&mut *transaction)
    .await?;

    let playlist = sqlx::query_as::<_, Playlist>(
        "INSERT INTO playlists (spotify_id, name, description, image_url, owner_name, imported_by, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, TRUE) \
         ON CONFLICT (spotify_id) DO UPDATE SET \
            name = EXCLUDED.name, description = EXCLUDED.description, \
            image_url = EXCLUDED.image_url, owner_name = EXCLUDED.owner_name, \
            imported_by = EXCLUDED.imported_by, imported_at = now(), is_active = TRUE \
         RETURNING id, spotify_id, name, description, image_url, owner_name, imported_by, \
                   imported_at, reveal_state, is_active",
    )
    .bind(&imported.spotify_id)
    .bind(&imported.name)
    .bind(imported.description.as_deref())
    .bind(imported.image_url.as_deref())
    .bind(imported.owner_name.as_deref())
    .bind(admin_id)
    .fetch_one(&mut *transaction)
    .await?;

    // Upsert tracks in batches. Existing ratings and vote counts are preserved.
    for chunk in imported.tracks.chunks(500) {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO tracks \
                (playlist_id, spotify_id, title, artists, album, album_image_url, preview_url, duration_ms, position) ",
        );

        builder.push_values(chunk, |mut row, track| {
            row.push_bind(playlist.id)
                .push_bind(track.spotify_id.as_str())
                .push_bind(track.title.as_str())
                .push_bind(track.artists.as_str())
                .push_bind(track.album.as_deref())
                .push_bind(track.album_image_url.as_deref())
                .push_bind(track.preview_url.as_deref())
                .push_bind(track.duration_ms)
                .push_bind(track.position);
        });

        builder.push(
            " ON CONFLICT (playlist_id, spotify_id) DO UPDATE SET \
                title = EXCLUDED.title, artists = EXCLUDED.artists, album = EXCLUDED.album, \
                album_image_url = EXCLUDED.album_image_url, duration_ms = EXCLUDED.duration_ms, \
                preview_url = COALESCE(EXCLUDED.preview_url, tracks.preview_url), \
                position = EXCLUDED.position",
        );

        builder.build().execute(&mut *transaction).await?;
    }

    let total_tracks =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&mut *transaction)
            .await?;

    transaction.commit().await?;

    Ok((playlist, total_tracks))
}
