use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::Playlist;

/// The single playlist currently set or open for voting.
pub async fn active_playlist(pool: &PgPool) -> Result<Option<Playlist>, sqlx::Error> {
    sqlx::query_as::<_, Playlist>(
        "SELECT id, spotify_id, name, description, image_url, owner_name, imported_by, \
                imported_at, reveal_state, is_active, scheduled_start_at, started_at, \
                voting_paused, voting_paused_at, scheduled_reveal_at, \
                pause_voting_on_reveal, start_error, start_attempts, \
                last_start_attempt_at, start_in_progress \
         FROM playlists WHERE is_active LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

/// Returns why voting is currently blocked, if it is.
pub fn voting_block_reason(playlist: &Playlist) -> Option<String> {
    voting_block_reason_parts(
        playlist.started_at,
        playlist.scheduled_start_at,
        playlist.voting_paused,
    )
}

pub fn voting_block_reason_parts(
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    scheduled_start_at: Option<chrono::DateTime<chrono::Utc>>,
    voting_paused: bool,
) -> Option<String> {
    if started_at.is_none() {
        return Some(match scheduled_start_at {
            Some(start) => format!(
                "voting hasn't started yet; it opens at {}",
                start.format("%Y-%m-%d %H:%M UTC")
            ),
            None => "voting hasn't started yet".to_string(),
        });
    }

    if voting_paused {
        return Some("voting is paused".to_string());
    }

    None
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
pub struct UserProgress {
    pub compared_pairs: i64,
    pub tracks_seen: i64,
    pub votes_cast: i64,
}

/// Pair, coverage and vote counters for one user in one playlist.
pub async fn user_progress(
    connection: &mut PgConnection,
    user_id: Uuid,
    playlist_id: Uuid,
) -> Result<UserProgress, sqlx::Error> {
    sqlx::query_as::<_, UserProgress>(
        "SELECT \
            (SELECT COUNT(*) FROM seen_pairs \
             WHERE user_id = $1 AND playlist_id = $2) AS compared_pairs, \
            (SELECT COUNT(*) FROM ( \
                SELECT track_low AS track_id FROM seen_pairs \
                WHERE user_id = $1 AND playlist_id = $2 \
                UNION \
                SELECT track_high FROM seen_pairs \
                WHERE user_id = $1 AND playlist_id = $2 \
            ) AS seen_tracks) AS tracks_seen, \
            (SELECT COUNT(*) FROM votes \
             WHERE user_id = $1 AND playlist_id = $2) AS votes_cast",
    )
    .bind(user_id)
    .bind(playlist_id)
    .fetch_one(connection)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn at(second: i64) -> chrono::DateTime<Utc> {
        Utc.timestamp_opt(second, 0).unwrap()
    }

    #[test]
    fn voting_is_unblocked_after_start() {
        assert_eq!(voting_block_reason_parts(Some(at(100)), None, false), None);
    }

    #[test]
    fn voting_without_a_start_is_blocked() {
        assert_eq!(
            voting_block_reason_parts(None, None, false).as_deref(),
            Some("voting hasn't started yet")
        );
    }

    #[test]
    fn voting_before_a_scheduled_start_names_the_time() {
        let reason = voting_block_reason_parts(None, Some(at(1_700_000_000)), false).unwrap();
        assert!(reason.starts_with("voting hasn't started yet; it opens at "));
        assert!(reason.ends_with(" UTC"));
    }

    #[test]
    fn a_paused_season_is_blocked_after_start() {
        assert_eq!(
            voting_block_reason_parts(Some(at(100)), None, true).as_deref(),
            Some("voting is paused")
        );
    }

    #[test]
    fn paused_before_start_reports_the_start_first() {
        assert_eq!(
            voting_block_reason_parts(None, None, true).as_deref(),
            Some("voting hasn't started yet")
        );
    }

    #[test]
    fn pause_only_matters_once_started() {
        assert_eq!(
            voting_block_reason_parts(Some(at(100)), Some(at(200)), true).as_deref(),
            Some("voting is paused")
        );
        assert_eq!(
            voting_block_reason_parts(Some(at(100)), Some(at(200)), false),
            None
        );
    }
}
