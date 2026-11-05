use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::spotify::SpotifyClient;

const CONCURRENCY: usize = 4;

/// Fetches missing preview MP3 URLs from public Spotify embed pages and writes
/// them to the database. Runs in the background so imports stay fast.
pub fn spawn(client: SpotifyClient, pool: PgPool, playlist_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = run(client, pool, playlist_id).await {
            tracing::error!(
                error = ?error,
                %playlist_id,
                "preview backfill failed"
            );
        }
    });
}

async fn run(client: SpotifyClient, pool: PgPool, playlist_id: Uuid) -> anyhow::Result<()> {
    let tracks = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, spotify_id FROM tracks \
         WHERE playlist_id = $1 AND preview_url IS NULL \
         ORDER BY position ASC, id ASC",
    )
    .bind(playlist_id)
    .fetch_all(&pool)
    .await?;

    let total = tracks.len();
    if total == 0 {
        tracing::debug!(%playlist_id, "preview backfill: nothing missing");
        return Ok(());
    }

    tracing::info!(%playlist_id, missing = total, "preview backfill started");

    let mut set = JoinSet::new();
    let mut updated = 0usize;
    let mut missed = 0usize;

    for (track_id, spotify_id) in tracks {
        while set.len() >= CONCURRENCY {
            if let Some(result) = set.join_next().await {
                match result {
                    Ok(true) => updated += 1,
                    Ok(false) => missed += 1,
                    Err(error) => {
                        missed += 1;
                        tracing::warn!(error = ?error, "preview backfill task failed");
                    }
                }
            }
        }

        let client = client.clone();
        let pool = pool.clone();

        set.spawn(async move {
            // Be gentle with Spotify's embed CDN.
            tokio::time::sleep(Duration::from_millis(50)).await;

            let Some(preview_url) = client.fetch_preview_url(&spotify_id).await else {
                return false;
            };

            let result = sqlx::query(
                "UPDATE tracks SET preview_url = $1 \
                 WHERE id = $2 AND preview_url IS NULL",
            )
            .bind(&preview_url)
            .bind(track_id)
            .execute(&pool)
            .await;

            match result {
                Ok(result) if result.rows_affected() > 0 => true,
                Ok(_) => false,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        %track_id,
                        "failed to save backfilled preview URL"
                    );
                    false
                }
            }
        });
    }

    while let Some(result) = set.join_next().await {
        match result {
            Ok(true) => updated += 1,
            Ok(false) => missed += 1,
            Err(error) => {
                missed += 1;
                tracing::warn!(error = ?error, "preview backfill task failed");
            }
        }
    }

    tracing::info!(
        %playlist_id,
        total,
        updated,
        missed,
        "preview backfill finished"
    );

    Ok(())
}
