use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::models::Playlist;

/// The single playlist currently open for voting, if one has been imported.
pub async fn active_playlist(pool: &PgPool) -> Result<Option<Playlist>, sqlx::Error> {
    sqlx::query_as::<_, Playlist>(
        "SELECT id, spotify_id, name, description, image_url, owner_name, imported_by, \
                imported_at, reveal_state, is_active \
         FROM playlists WHERE is_active LIMIT 1",
    )
    .fetch_optional(pool)
    .await
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

