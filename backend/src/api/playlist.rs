use std::time::Instant;

use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::api::spotify::map_error;
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::metadata;
use crate::metadata_backfill;
use crate::models::{Playlist, Track};
use crate::spotify::{self, ImportedPlaylist, ImportedPlaylistInfo, SpotifyClient, SpotifyError};
use crate::spotify_auth;
use crate::state::AppState;

const PLAYLIST_FIELDS: &str = "id, spotify_id, name, description, image_url, owner_name, \
    imported_by, imported_at, reveal_state, is_active, scheduled_start_at, started_at, \
    voting_paused, voting_paused_at, scheduled_reveal_at, pause_voting_on_reveal, \
    start_error, start_attempts, last_start_attempt_at, start_in_progress";

#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    playlist: String,
}

#[derive(Debug, Serialize)]
pub struct ImportSummary {
    playlist: Playlist,
    imported_tracks: usize,
    skipped_tracks: usize,
    pub(crate) total_tracks: i64,
    previews_missing: i64,
    metadata_backfill_started: bool,
}

#[derive(Debug, Serialize)]
pub struct PlaylistOverview {
    playlist: Option<Playlist>,
    track_count: i64,
    previews_missing: i64,
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
    let spotify_id = parse_playlist_input(&payload.playlist)?;

    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify import is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;

    let started = Instant::now();
    tracing::info!(
        admin_id = %admin.0.id,
        spotify_playlist_id = %spotify_id,
        "playlist import started"
    );

    let connected = spotify_auth::is_connected(&state.db).await.unwrap_or(false);
    let (imported, connected_mode) = fetch_imported_playlist(client, &state.db, &spotify_id)
        .await
        .map_err(|error| map_error(error, connected))?;
    let connected = connected_mode;

    tracing::info!(
        admin_id = %admin.0.id,
        spotify_playlist_id = %imported.spotify_id,
        name = %imported.name,
        tracks = imported.tracks.len(),
        skipped = imported.skipped_tracks,
        auth_mode = if connected { "oauth" } else { "client_credentials" },
        elapsed_ms = started.elapsed().as_millis() as u64,
        "Spotify playlist fetched"
    );

    let summary = finish_import(client, &state.db, &imported, Some(admin.0.id)).await?;
    state.cache.clear();
    Ok(Json(summary))
}

pub(crate) async fn fetch_imported_playlist(
    client: &SpotifyClient,
    pool: &PgPool,
    spotify_id: &str,
) -> Result<(ImportedPlaylist, bool), SpotifyError> {
    match spotify_auth::user_access_token(client, pool).await? {
        Some(token) => {
            match client.fetch_playlist_with_token(spotify_id, &token).await {
                Ok(playlist) => Ok((playlist, true)),
                Err(SpotifyError::Unauthorized) => {
                    // A user token can be rejected before its recorded expiry.
                    // Refresh once, then retry; do not report this as a bad app
                    // client secret.
                    let token = spotify_auth::force_refresh_user_token(client, pool).await?;
                    match client.fetch_playlist_with_token(spotify_id, &token).await {
                        Ok(playlist) => Ok((playlist, true)),
                        Err(SpotifyError::Unauthorized) => Err(SpotifyError::Reconnect),
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        }
        None => Ok((client.fetch_playlist(spotify_id).await?, false)),
    }
}

pub(crate) async fn fetch_playlist_info(
    client: &SpotifyClient,
    pool: &PgPool,
    spotify_id: &str,
) -> Result<(ImportedPlaylistInfo, bool), SpotifyError> {
    match spotify_auth::user_access_token(client, pool).await? {
        Some(token) => {
            match client
                .fetch_playlist_info_with_token(spotify_id, &token)
                .await
            {
                Ok(info) => Ok((info, true)),
                Err(SpotifyError::Unauthorized) => {
                    let token = spotify_auth::force_refresh_user_token(client, pool).await?;
                    match client
                        .fetch_playlist_info_with_token(spotify_id, &token)
                        .await
                    {
                        Ok(info) => Ok((info, true)),
                        Err(SpotifyError::Unauthorized) => Err(SpotifyError::Reconnect),
                        Err(error) => Err(error),
                    }
                }
                Err(error) => Err(error),
            }
        }
        None => Ok((client.fetch_playlist_info(spotify_id).await?, false)),
    }
}

pub(crate) async fn finish_import(
    client: &SpotifyClient,
    pool: &PgPool,
    imported: &ImportedPlaylist,
    admin_id: Option<Uuid>,
) -> Result<ImportSummary, ApiError> {
    let (playlist, total_tracks) = persist_import(pool, admin_id, imported).await?;

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
    .fetch_one(pool)
    .await?;

    if previews_missing > 0 {
        crate::preview_backfill::spawn(client.clone(), pool.clone(), playlist.id);
        tracing::info!(
            playlist_id = %playlist.id,
            missing = previews_missing,
            "started preview backfill after import"
        );
    }

    // Link tracks to artists immediately so the first import already has
    // artist/album aggregations; genres arrive with the backfill below. A
    // metadata failure must not turn a successful import into an error.
    if let Err(error) = metadata::sync_track_artists(pool, playlist.id, &imported.tracks).await {
        tracing::error!(
            playlist_id = %playlist.id,
            error = ?error,
            "could not link imported tracks to artists"
        );
    }

    let metadata_backfill_started =
        match metadata_backfill::start(client.clone(), pool.clone(), playlist.id).await {
            Ok((run_id, spawned)) => {
                tracing::info!(
                    playlist_id = %playlist.id,
                    %run_id,
                    spawned,
                    "metadata backfill queued after import"
                );
                spawned
            }
            Err(error) => {
                tracing::error!(
                    playlist_id = %playlist.id,
                    error = ?error,
                    "could not start metadata backfill after import"
                );
                false
            }
        };

    Ok(ImportSummary {
        playlist,
        imported_tracks: imported.tracks.len(),
        skipped_tracks: imported.skipped_tracks,
        total_tracks,
        previews_missing,
        metadata_backfill_started,
    })
}

fn parse_playlist_input(input: &str) -> Result<String, ApiError> {
    spotify::parse_playlist_id(input).ok_or_else(|| {
        ApiError::bad_request("expected a Spotify playlist URL, URI, or 22-character playlist ID")
    })
}

#[derive(Debug, Deserialize)]
pub struct SchedulePlaylistRequest {
    playlist: String,
    start_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct VotingPauseRequest {
    paused: bool,
}

#[derive(Debug, Serialize)]
pub struct CancelScheduleResponse {
    cancelled: bool,
}

#[derive(Debug, Serialize)]
pub struct VotingPauseResponse {
    voting_paused: bool,
}

pub async fn schedule_playlist(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<SchedulePlaylistRequest>,
) -> Result<Json<Playlist>, ApiError> {
    let spotify_id = parse_playlist_input(&payload.playlist)?;

    if payload.start_at <= Utc::now() {
        return Err(ApiError::bad_request(
            "the scheduled start must be in the future; use force start for an immediate start",
        ));
    }

    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;

    let connected = spotify_auth::is_connected(&state.db).await.unwrap_or(false);
    let (info, connected_mode) = fetch_playlist_info(client, &state.db, &spotify_id)
        .await
        .map_err(|error| map_error(error, connected))?;
    let connected = connected_mode;
    tracing::info!(
        admin_id = %admin.0.id,
        spotify_playlist_id = %info.spotify_id,
        name = %info.name,
        start_at = %payload.start_at,
        auth_mode = if connected { "oauth" } else { "client_credentials" },
        "playlist scheduled"
    );

    let mut transaction = state.db.begin().await?;

    let existing = sqlx::query_as::<_, Playlist>(&format!(
        "SELECT {PLAYLIST_FIELDS} FROM playlists WHERE is_active FOR UPDATE"
    ))
    .fetch_optional(&mut *transaction)
    .await?;

    if let Some(current) = existing {
        if current.started_at.is_some() {
            return Err(ApiError::Conflict(
                "a playlist is already loaded; full reset or remove it before scheduling another"
                    .to_string(),
            ));
        }

        if current.start_in_progress {
            return Err(ApiError::Conflict(
                "this scheduled playlist is currently being started; try again in a moment"
                    .to_string(),
            ));
        }

        if current
            .scheduled_reveal_at
            .is_some_and(|reveal| reveal < payload.start_at)
        {
            return Err(ApiError::Conflict(
                "the scheduled reveal is before the new start; reschedule or cancel the reveal first"
                    .to_string(),
            ));
        }

        if current.spotify_id == spotify_id {
            let updated = sqlx::query_as::<_, Playlist>(&format!(
                "UPDATE playlists SET \
                    name = $2, description = $3, image_url = $4, owner_name = $5, \
                    scheduled_start_at = $6, started_at = NULL, start_error = NULL, \
                    start_attempts = 0, last_start_attempt_at = NULL, start_in_progress = FALSE \
                 WHERE id = $1 \
                 RETURNING {PLAYLIST_FIELDS}"
            ))
            .bind(current.id)
            .bind(&info.name)
            .bind(info.description.as_deref())
            .bind(info.image_url.as_deref())
            .bind(info.owner_name.as_deref())
            .bind(payload.start_at)
            .fetch_one(&mut *transaction)
            .await?;

            transaction.commit().await?;
            state.cache.clear();
            return Ok(Json(updated));
        }

        // Different playlist, not started yet: replace the empty scheduled row.
        sqlx::query("DELETE FROM playlists WHERE id = $1")
            .bind(current.id)
            .execute(&mut *transaction)
            .await?;
    }

    // Never resurrect an old playlist row with the same Spotify ID. This keeps
    // a new season free of old tracks, votes and duels.
    let removed = sqlx::query("DELETE FROM playlists WHERE spotify_id = $1")
        .bind(&spotify_id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();
    if removed > 0 {
        tracing::warn!(
            spotify_playlist_id = %spotify_id,
            removed,
            "removed old playlist rows before scheduling a fresh season"
        );
    }

    let playlist = sqlx::query_as::<_, Playlist>(&format!(
        "INSERT INTO playlists \
            (spotify_id, name, description, image_url, owner_name, imported_by, is_active, \
             reveal_state, scheduled_start_at, started_at, voting_paused) \
         VALUES ($1, $2, $3, $4, $5, $6, TRUE, 'hidden', $7, NULL, FALSE) \
         RETURNING {PLAYLIST_FIELDS}"
    ))
    .bind(&info.spotify_id)
    .bind(&info.name)
    .bind(info.description.as_deref())
    .bind(info.image_url.as_deref())
    .bind(info.owner_name.as_deref())
    .bind(admin.0.id)
    .bind(payload.start_at)
    .fetch_one(&mut *transaction)
    .await?;

    transaction.commit().await?;
    state.cache.clear();

    tracing::info!(
        playlist_id = %playlist.id,
        spotify_playlist_id = %playlist.spotify_id,
        start_at = %payload.start_at,
        "playlist schedule created"
    );

    Ok(Json(playlist))
}

pub async fn cancel_scheduled_playlist(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<CancelScheduleResponse>, ApiError> {
    let mut transaction = state.db.begin().await?;
    let playlist = sqlx::query_as::<_, Playlist>(&format!(
        "SELECT {PLAYLIST_FIELDS} FROM playlists WHERE is_active FOR UPDATE"
    ))
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| ApiError::NotFound("no playlist is scheduled".to_string()))?;

    if playlist.started_at.is_some() {
        return Err(ApiError::Conflict(
            "this playlist is already loaded; remove it or use full reset".to_string(),
        ));
    }

    if playlist.start_in_progress {
        return Err(ApiError::Conflict(
            "a start attempt is in progress; try again in a moment".to_string(),
        ));
    }

    sqlx::query("DELETE FROM playlists WHERE id = $1")
        .bind(playlist.id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    state.cache.clear();

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.id,
        "scheduled playlist cancelled"
    );

    Ok(Json(CancelScheduleResponse { cancelled: true }))
}

pub async fn start_playlist_now(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<ImportSummary>, ApiError> {
    let playlist = active_playlist(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("no playlist is scheduled".to_string()))?;

    if playlist.started_at.is_some() {
        return Err(ApiError::Conflict(
            "this playlist is already started".to_string(),
        ));
    }

    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;

    let claimed = sqlx::query_scalar::<_, Uuid>(
        "UPDATE playlists SET start_in_progress = TRUE, \
                last_start_attempt_at = now(), start_attempts = start_attempts + 1 \
         WHERE id = $1 AND start_in_progress = FALSE \
         RETURNING id",
    )
    .bind(playlist.id)
    .fetch_optional(&state.db)
    .await?;

    if claimed.is_none() {
        return Err(ApiError::Conflict(
            "a start attempt is already in progress; try again in a moment".to_string(),
        ));
    }

    let result = async {
        let connected = spotify_auth::is_connected(&state.db).await.unwrap_or(false);
        let (imported, _) = fetch_imported_playlist(client, &state.db, &playlist.spotify_id)
            .await
            .map_err(|error| map_error(error, connected))?;
        finish_import(client, &state.db, &imported, playlist.imported_by).await
    }
    .await;

    match result {
        Ok(summary) => {
            sqlx::query(
                "UPDATE playlists SET start_in_progress = FALSE, start_error = NULL WHERE id = $1",
            )
            .bind(playlist.id)
            .execute(&state.db)
            .await?;
            state.cache.clear();

            tracing::info!(
                admin_id = %admin.0.id,
                playlist_id = %playlist.id,
                "playlist force-started"
            );

            Ok(Json(summary))
        }
        Err(error) => {
            let message = error.to_string();
            let _ = sqlx::query(
                "UPDATE playlists SET start_in_progress = FALSE, start_error = $2 WHERE id = $1",
            )
            .bind(playlist.id)
            .bind(&message)
            .execute(&state.db)
            .await;
            Err(error)
        }
    }
}

pub async fn set_voting_paused(
    State(state): State<AppState>,
    admin: AdminUser,
    Json(payload): Json<VotingPauseRequest>,
) -> Result<Json<VotingPauseResponse>, ApiError> {
    let playlist = active_playlist(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("no playlist has been set yet".to_string()))?;

    sqlx::query(
        "UPDATE playlists SET voting_paused = $2, \
                voting_paused_at = CASE WHEN $2 THEN now() ELSE NULL END \
         WHERE id = $1",
    )
    .bind(playlist.id)
    .bind(payload.paused)
    .execute(&state.db)
    .await?;

    state.cache.clear();

    tracing::info!(
        admin_id = %admin.0.id,
        playlist_id = %playlist.id,
        paused = payload.paused,
        "voting pause changed"
    );

    Ok(Json(VotingPauseResponse {
        voting_paused: payload.paused,
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
        crate::preview_backfill::spawn(client.clone(), state.db.clone(), playlist.id);
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MetadataBackfillRun {
    id: Uuid,
    status: String,
    total_tracks: i32,
    processed_tracks: i32,
    total_artists: i32,
    processed_artists: i32,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MetadataStatus {
    playlist_id: Option<Uuid>,
    tracks_total: i64,
    tracks_with_year: i64,
    tracks_with_genre: i64,
    artists_total: i64,
    latest_run: Option<MetadataBackfillRun>,
}

#[derive(Debug, Serialize)]
pub struct BackfillMetadataResponse {
    run_id: Uuid,
    started: bool,
}

pub async fn metadata_status(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<MetadataStatus>, ApiError> {
    metadata_backfill::reap_stale_runs(&state.db).await?;

    let Some(playlist) = active_playlist(&state.db).await? else {
        return Ok(Json(MetadataStatus {
            playlist_id: None,
            tracks_total: 0,
            tracks_with_year: 0,
            tracks_with_genre: 0,
            artists_total: 0,
            latest_run: None,
        }));
    };

    #[derive(Debug, sqlx::FromRow)]
    struct MetadataCounts {
        tracks_total: i64,
        tracks_with_year: i64,
        tracks_with_genre: i64,
        artists_total: i64,
    }

    let counts = sqlx::query_as::<_, MetadataCounts>(
        "SELECT \
            COUNT(*) AS tracks_total, \
            COUNT(release_year) AS tracks_with_year, \
            COUNT(primary_genre) AS tracks_with_genre, \
            (SELECT COUNT(DISTINCT ta.artist_id) \
             FROM track_artists ta JOIN tracks t ON t.id = ta.track_id \
             WHERE t.playlist_id = $1) AS artists_total \
         FROM tracks WHERE playlist_id = $1",
    )
    .bind(playlist.id)
    .fetch_one(&state.db)
    .await?;

    let latest_run = sqlx::query_as::<_, MetadataBackfillRun>(
        "SELECT id, status, total_tracks, processed_tracks, total_artists, \
                processed_artists, started_at, finished_at, error \
         FROM metadata_backfill_runs WHERE playlist_id = $1 \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(playlist.id)
    .fetch_optional(&state.db)
    .await?;

    Ok(Json(MetadataStatus {
        playlist_id: Some(playlist.id),
        tracks_total: counts.tracks_total,
        tracks_with_year: counts.tracks_with_year,
        tracks_with_genre: counts.tracks_with_genre,
        artists_total: counts.artists_total,
        latest_run,
    }))
}

pub async fn backfill_metadata(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<BackfillMetadataResponse>, ApiError> {
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

    let (run_id, started) =
        metadata_backfill::start(client.clone(), state.db.clone(), playlist.id).await?;

    tracing::info!(
        playlist_id = %playlist.id,
        %run_id,
        started,
        "manual metadata backfill requested"
    );

    Ok(Json(BackfillMetadataResponse { run_id, started }))
}

pub async fn overview(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<PlaylistOverview>, ApiError> {
    let playlist = active_playlist(&state.db).await?;

    let (track_count, previews_missing) = match &playlist {
        Some(playlist) => {
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT COUNT(*), COUNT(*) FILTER (WHERE preview_url IS NULL) \
             FROM tracks WHERE playlist_id = $1",
            )
            .bind(playlist.id)
            .fetch_one(&state.db)
            .await?
        }
        None => (0, 0),
    };

    Ok(Json(PlaylistOverview {
        playlist,
        track_count,
        previews_missing,
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

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
        .bind(playlist.id)
        .fetch_one(&state.db)
        .await?;

    let tracks = sqlx::query_as::<_, Track>(
        "SELECT id, playlist_id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, release_date, release_date_precision, release_year, decade, album_id, \
                album_type, total_tracks, disc_number, track_number, primary_genre, position, \
                rating, comparisons, wins \
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
    admin_id: Option<Uuid>,
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
        "INSERT INTO playlists \
            (spotify_id, name, description, image_url, owner_name, imported_by, is_active, \
             scheduled_start_at, started_at) \
         VALUES ($1, $2, $3, $4, $5, $6, TRUE, now(), now()) \
         ON CONFLICT (spotify_id) DO UPDATE SET \
            name = EXCLUDED.name, description = EXCLUDED.description, \
            image_url = EXCLUDED.image_url, owner_name = EXCLUDED.owner_name, \
            imported_by = EXCLUDED.imported_by, imported_at = now(), is_active = TRUE, \
            started_at = now(), \
            scheduled_start_at = COALESCE(playlists.scheduled_start_at, now()), \
            start_error = NULL \
         RETURNING id, spotify_id, name, description, image_url, owner_name, imported_by, \
                   imported_at, reveal_state, is_active, scheduled_start_at, started_at, \
                   voting_paused, voting_paused_at, scheduled_reveal_at, \
                   pause_voting_on_reveal, start_error, start_attempts, \
                   last_start_attempt_at, start_in_progress",
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
                (playlist_id, spotify_id, title, artists, album, album_image_url, preview_url, \
                 duration_ms, release_date, release_date_precision, release_year, decade, \
                 album_id, album_type, total_tracks, disc_number, track_number, position) ",
        );

        builder.push_values(chunk, |mut row, track| {
            let decade = track.release_year.map(metadata::decade_from_year);
            row.push_bind(playlist.id)
                .push_bind(track.spotify_id.as_str())
                .push_bind(track.title.as_str())
                .push_bind(track.artists.as_str())
                .push_bind(track.album.as_deref())
                .push_bind(track.album_image_url.as_deref())
                .push_bind(track.preview_url.as_deref())
                .push_bind(track.duration_ms)
                .push_bind(track.release_date.as_deref())
                .push_bind(track.release_date_precision.as_deref())
                .push_bind(track.release_year)
                .push_bind(decade)
                .push_bind(track.album_id.as_deref())
                .push_bind(track.album_type.as_deref())
                .push_bind(track.total_tracks)
                .push_bind(track.disc_number)
                .push_bind(track.track_number)
                .push_bind(track.position);
        });

        builder.push(
            " ON CONFLICT (playlist_id, spotify_id) DO UPDATE SET \
                title = EXCLUDED.title, artists = EXCLUDED.artists, album = EXCLUDED.album, \
                album_image_url = EXCLUDED.album_image_url, duration_ms = EXCLUDED.duration_ms, \
                preview_url = COALESCE(EXCLUDED.preview_url, tracks.preview_url), \
                release_date = EXCLUDED.release_date, \
                release_date_precision = EXCLUDED.release_date_precision, \
                release_year = EXCLUDED.release_year, decade = EXCLUDED.decade, \
                album_id = EXCLUDED.album_id, album_type = EXCLUDED.album_type, \
                total_tracks = EXCLUDED.total_tracks, disc_number = EXCLUDED.disc_number, \
                track_number = EXCLUDED.track_number, position = EXCLUDED.position",
        );

        builder.build().execute(&mut *transaction).await?;
    }

    // Tracks removed from Spotify are kept for history. Put them after the
    // freshly imported positions so a re-sync cannot leave duplicate positions.
    let imported_spotify_ids: Vec<String> = imported
        .tracks
        .iter()
        .map(|track| track.spotify_id.clone())
        .collect();
    let next_position = imported
        .tracks
        .iter()
        .map(|track| track.position)
        .max()
        .unwrap_or(-1)
        + 1;
    sqlx::query(
        "WITH retained AS ( \
             SELECT id, ROW_NUMBER() OVER (ORDER BY position, id) AS rn \
             FROM tracks \
             WHERE playlist_id = $1 \
               AND NOT (spotify_id = ANY($2)) \
         ) \
         UPDATE tracks t \
         SET position = $3::int + (retained.rn - 1)::int \
         FROM retained WHERE t.id = retained.id",
    )
    .bind(playlist.id)
    .bind(&imported_spotify_ids)
    .bind(next_position)
    .execute(&mut *transaction)
    .await?;

    let total_tracks =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tracks WHERE playlist_id = $1")
            .bind(playlist.id)
            .fetch_one(&mut *transaction)
            .await?;

    transaction.commit().await?;

    Ok((playlist, total_tracks))
}
