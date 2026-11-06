//! Background schedule runner: fetches a playlist when its start time arrives
//! and opens The Reveal when its scheduled time arrives.

use std::time::Duration;

use chrono::Utc;
use sqlx::PgPool;
use tokio::time::MissedTickBehavior;
use uuid::Uuid;

use crate::api::playlist::{fetch_imported_playlist, finish_import};
use crate::error::ApiError;
use crate::state::AppState;

const TICK: Duration = Duration::from_secs(15);
const RETRY_AFTER_SECONDS: i64 = 60;
const STALE_START_SECONDS: i64 = 600;

#[derive(Debug, sqlx::FromRow)]
struct DuePlaylist {
    id: Uuid,
    spotify_id: String,
    imported_by: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct DueReveal {
    id: Uuid,
    pause_voting_on_reveal: bool,
}

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(TICK);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(error) = tick(&state).await {
                tracing::error!(error = ?error, "schedule runner tick failed");
            }
        }
    });
}

async fn tick(state: &AppState) -> anyhow::Result<()> {
    clear_stale_start(state).await?;
    process_due_start(state).await?;
    process_due_reveal(state).await?;
    Ok(())
}

async fn clear_stale_start(state: &AppState) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE playlists SET start_in_progress = FALSE \
         WHERE start_in_progress = TRUE \
           AND (last_start_attempt_at IS NULL \
                OR last_start_attempt_at < now() - make_interval(secs => $1))",
    )
    .bind(STALE_START_SECONDS as f64)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn process_due_start(state: &AppState) -> anyhow::Result<()> {
    let due = sqlx::query_as::<_, DuePlaylist>(
        "SELECT id, spotify_id, imported_by \
         FROM playlists \
         WHERE is_active = TRUE AND started_at IS NULL \
           AND scheduled_start_at IS NOT NULL AND scheduled_start_at <= now() \
           AND start_in_progress = FALSE \
           AND (last_start_attempt_at IS NULL \
                OR last_start_attempt_at < now() - make_interval(secs => $1)) \
         ORDER BY scheduled_start_at ASC LIMIT 1",
    )
    .bind(RETRY_AFTER_SECONDS as f64)
    .fetch_optional(&state.db)
    .await?;

    let Some(due) = due else {
        return Ok(());
    };

    let claimed = sqlx::query_scalar::<_, Uuid>(
        "UPDATE playlists SET start_in_progress = TRUE, \
                last_start_attempt_at = now(), start_attempts = start_attempts + 1 \
         WHERE id = $1 AND start_in_progress = FALSE \
         RETURNING id",
    )
    .bind(due.id)
    .fetch_optional(&state.db)
    .await?;

    if claimed.is_none() {
        return Ok(());
    }

    let Some(client) = state.spotify.as_ref() else {
        fail_start(&state.db, due.id, "Spotify is not configured").await;
        tracing::warn!(
            playlist_id = %due.id,
            "scheduled start is waiting for Spotify to be configured"
        );
        return Ok(());
    };

    let result: Result<_, ApiError> = async {
        let (imported, _) = fetch_imported_playlist(client, &state.db, &due.spotify_id)
            .await
            .map_err(|error| ApiError::service_unavailable(format!("{error:#}")))?;
        finish_import(client, &state.db, &imported, due.imported_by).await
    }
    .await;

    match result {
        Ok(summary) => {
            sqlx::query(
                "UPDATE playlists SET start_in_progress = FALSE, start_error = NULL WHERE id = $1",
            )
            .bind(due.id)
            .execute(&state.db)
            .await?;
            state.cache.clear();

            tracing::info!(
                playlist_id = %due.id,
                spotify_playlist_id = %due.spotify_id,
                tracks = summary.total_tracks,
                started_at = %Utc::now(),
                "scheduled playlist started"
            );
        }
        Err(error) => {
            let message = error.to_string();
            fail_start(&state.db, due.id, &message).await;
            tracing::warn!(
                playlist_id = %due.id,
                spotify_playlist_id = %due.spotify_id,
                error = %message,
                "scheduled playlist start failed; will retry"
            );
        }
    }

    Ok(())
}

async fn fail_start(pool: &PgPool, playlist_id: Uuid, message: &str) {
    let _ = sqlx::query(
        "UPDATE playlists SET start_in_progress = FALSE, start_error = $2 WHERE id = $1",
    )
    .bind(playlist_id)
    .bind(message)
    .execute(pool)
    .await;
}

async fn process_due_reveal(state: &AppState) -> anyhow::Result<()> {
    let due = sqlx::query_as::<_, DueReveal>(
        "SELECT id, pause_voting_on_reveal \
         FROM playlists \
         WHERE is_active = TRUE AND started_at IS NOT NULL \
           AND reveal_state = 'hidden' \
           AND scheduled_reveal_at IS NOT NULL AND scheduled_reveal_at <= now() \
         ORDER BY scheduled_reveal_at ASC LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await?;

    let Some(due) = due else {
        return Ok(());
    };

    let updated = sqlx::query(
        "UPDATE playlists SET \
            reveal_state = 'revealed', \
            scheduled_reveal_at = NULL, \
            voting_paused = CASE WHEN $2 THEN TRUE ELSE voting_paused END, \
            voting_paused_at = CASE WHEN $2 THEN now() ELSE voting_paused_at END, \
            pause_voting_on_reveal = FALSE \
         WHERE id = $1 AND reveal_state = 'hidden'",
    )
    .bind(due.id)
    .bind(due.pause_voting_on_reveal)
    .execute(&state.db)
    .await?;

    if updated.rows_affected() > 0 {
        state.cache.clear();
        tracing::info!(
            playlist_id = %due.id,
            paused = due.pause_voting_on_reveal,
            "scheduled reveal opened"
        );
    }

    Ok(())
}
