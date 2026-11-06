use std::collections::HashSet;
use std::time::Instant;

use sqlx::PgPool;
use uuid::Uuid;

use crate::metadata;
use crate::spotify::{SpotifyClient, SpotifyError};
use crate::spotify_auth;

const TRACK_CHUNK: usize = 200;

/// Mark every in-progress run as failed. Called once at startup: no task from
/// the previous process can still be alive, so leaving the rows as `running`
/// would permanently block the admin retry button.
pub async fn fail_orphaned_runs(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE metadata_backfill_runs \
         SET status = 'failed', \
             error = COALESCE(error, 'orphaned run: the backend restarted before it finished'), \
             finished_at = now() \
         WHERE status = 'running'",
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::warn!(
            runs = result.rows_affected(),
            "marked orphaned metadata backfill runs as failed"
        );
    }

    Ok(result.rows_affected())
}

/// A run that has not finished within this window is assumed to be stuck in
/// this process. Marking it failed lets the admin retry instead of staring at a
/// permanent "Backfilling…" state.
pub async fn reap_stale_runs(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE metadata_backfill_runs \
         SET status = 'failed', \
             error = COALESCE(error, 'stale run: the backend restarted before it finished'), \
             finished_at = now() \
         WHERE status = 'running' \
           AND started_at < now() - interval '30 minutes'",
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        tracing::warn!(
            runs = result.rows_affected(),
            "marked stale metadata backfill runs as failed"
        );
    }

    Ok(result.rows_affected())
}

/// Start (or reuse) a metadata backfill run. The boolean reports whether a
/// new run was actually spawned.
pub async fn start(
    client: SpotifyClient,
    pool: PgPool,
    playlist_id: Uuid,
) -> Result<(Uuid, bool), sqlx::Error> {
    reap_stale_runs(&pool).await?;

    let running = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM metadata_backfill_runs \
         WHERE playlist_id = $1 AND status = 'running' \
         ORDER BY started_at DESC LIMIT 1",
    )
    .bind(playlist_id)
    .fetch_optional(&pool)
    .await?;

    if let Some(run_id) = running {
        tracing::info!(%playlist_id, %run_id, "metadata backfill already running");
        return Ok((run_id, false));
    }

    let run_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO metadata_backfill_runs (playlist_id) VALUES ($1) RETURNING id",
    )
    .bind(playlist_id)
    .fetch_one(&pool)
    .await?;

    tracing::info!(%playlist_id, %run_id, "metadata backfill started");

    tokio::spawn(async move {
        if let Err(error) = run(&client, &pool, playlist_id, run_id).await {
            tracing::error!(error = ?error, %playlist_id, %run_id, "metadata backfill failed");
            let message = format!("{error:#}");
            let _ = sqlx::query(
                "UPDATE metadata_backfill_runs \
                 SET status = 'failed', error = $2, finished_at = now() WHERE id = $1",
            )
            .bind(run_id)
            .bind(message)
            .execute(&pool)
            .await;
        }
    });

    Ok((run_id, true))
}

async fn run(
    client: &SpotifyClient,
    pool: &PgPool,
    playlist_id: Uuid,
    run_id: Uuid,
) -> anyhow::Result<()> {
    let started = Instant::now();

    let spotify_id =
        sqlx::query_scalar::<_, String>("SELECT spotify_id FROM playlists WHERE id = $1")
            .bind(playlist_id)
            .fetch_one(pool)
            .await?;

    let user_token = spotify_auth::user_access_token(client, pool).await?;
    let (imported, token) = match user_token {
        Some(token) => (
            client
                .fetch_playlist_with_token(&spotify_id, &token)
                .await?,
            token,
        ),
        None => {
            let token = client.client_credentials_token().await?;
            let imported = client
                .fetch_playlist_with_token(&spotify_id, &token)
                .await?;
            (imported, token)
        }
    };

    let mut artist_ids = HashSet::new();
    for track in &imported.tracks {
        for artist in &track.artist_refs {
            if let Some(id) = &artist.spotify_id {
                artist_ids.insert(id.clone());
            }
        }
    }
    let unique_artist_ids: Vec<String> = artist_ids.into_iter().collect();

    sqlx::query(
        "UPDATE metadata_backfill_runs \
         SET total_tracks = $2, total_artists = $3 WHERE id = $1",
    )
    .bind(run_id)
    .bind(imported.tracks.len() as i32)
    .bind(unique_artist_ids.len() as i32)
    .execute(pool)
    .await?;

    tracing::info!(
        %playlist_id,
        %run_id,
        tracks = imported.tracks.len(),
        artists = unique_artist_ids.len(),
        "metadata backfill fetched playlist"
    );

    let mut processed_tracks = 0usize;
    for chunk in imported.tracks.chunks(TRACK_CHUNK) {
        metadata::apply_track_metadata(pool, playlist_id, chunk).await?;
        processed_tracks += chunk.len();

        sqlx::query("UPDATE metadata_backfill_runs SET processed_tracks = $2 WHERE id = $1")
            .bind(run_id)
            .bind(processed_tracks as i32)
            .execute(pool)
            .await?;

        tracing::debug!(
            %playlist_id,
            %run_id,
            processed_tracks,
            total_tracks = imported.tracks.len(),
            "metadata backfill progress"
        );
    }

    metadata::sync_track_artists(pool, playlist_id, &imported.tracks).await?;

    let mut processed_artists = 0usize;
    let mut genres_available = false;
    let mut artist_lookup_disabled = false;

    for chunk in unique_artist_ids.chunks(50) {
        if !artist_lookup_disabled {
            match client.fetch_artists(&token, chunk).await {
                Ok(artists) => {
                    metadata::update_artist_genres(pool, &artists).await?;
                    genres_available = true;
                }
                Err(SpotifyError::Forbidden) | Err(SpotifyError::NotFound) => {
                    // Spotify no longer exposes artist genres to every app.
                    // Keep the already-stored metadata and finish without them.
                    artist_lookup_disabled = true;
                    tracing::warn!(
                        %playlist_id,
                        %run_id,
                        "Spotify rejected the artist genre lookup; completing without genres"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        %playlist_id,
                        %run_id,
                        error = ?error,
                        "artist genre lookup failed; continuing without this batch"
                    );
                }
            }
        }

        processed_artists += chunk.len();

        sqlx::query("UPDATE metadata_backfill_runs SET processed_artists = $2 WHERE id = $1")
            .bind(run_id)
            .bind(processed_artists as i32)
            .execute(pool)
            .await?;

        tracing::debug!(
            %playlist_id,
            %run_id,
            processed_artists,
            total_artists = unique_artist_ids.len(),
            "artist genre backfill progress"
        );
    }

    if genres_available || unique_artist_ids.is_empty() {
        metadata::refresh_track_genres(pool, playlist_id).await?;
    } else {
        // Do not wipe genres from an earlier successful run when the artist
        // endpoint is unavailable.
        tracing::warn!(
            %playlist_id,
            %run_id,
            "no artist genres were returned; keeping existing track genres"
        );
    }

    sqlx::query(
        "UPDATE metadata_backfill_runs \
         SET status = 'completed', processed_tracks = $2, processed_artists = $3, \
             finished_at = now(), error = NULL \
         WHERE id = $1",
    )
    .bind(run_id)
    .bind(imported.tracks.len() as i32)
    .bind(unique_artist_ids.len() as i32)
    .execute(pool)
    .await?;

    tracing::info!(
        %playlist_id,
        %run_id,
        tracks = imported.tracks.len(),
        artists = unique_artist_ids.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "metadata backfill finished"
    );

    Ok(())
}
