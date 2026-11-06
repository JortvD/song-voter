//! Small "liner note" stories derived from the active playlist: the
//! most-duelled song and the top voter.

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::{MatchupTrack, Playlist};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ChartStories {
    pub playlist_id: Uuid,
    pub playlist_name: String,
    pub revealed: bool,
    pub most_duelled: Option<DuelledStory>,
    pub top_voter: Option<TopVoterStory>,
}

#[derive(Debug, Serialize)]
pub struct DuelledStory {
    pub track: MatchupTrack,
    pub comparisons: i32,
}

#[derive(Debug, Serialize)]
pub struct TopVoterStory {
    pub username: String,
    pub picks: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct StoryTrackRow {
    id: Uuid,
    spotify_id: String,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    preview_url: Option<String>,
    duration_ms: Option<i32>,
    comparisons: i32,
}

impl StoryTrackRow {
    fn into_track(self) -> MatchupTrack {
        MatchupTrack {
            id: self.id,
            spotify_id: self.spotify_id,
            title: self.title,
            artists: self.artists,
            album: self.album,
            album_image_url: self.album_image_url,
            preview_url: self.preview_url,
            duration_ms: self.duration_ms,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TopVoterRow {
    username: String,
    picks: i64,
}

/// Public stories. Gated by The Reveal, like the standings themselves.
pub async fn public(State(state): State<AppState>) -> Result<Json<ChartStories>, ApiError> {
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

    Ok(Json(load_stories(&state.db, &playlist).await?))
}

/// Admin stories, available even while the standings are sealed.
pub async fn admin(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<ChartStories>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    Ok(Json(load_stories(&state.db, &playlist).await?))
}

pub async fn load_stories(
    pool: &PgPool,
    playlist: &Playlist,
) -> Result<ChartStories, ApiError> {
    let most_duelled = sqlx::query_as::<_, StoryTrackRow>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, comparisons \
         FROM tracks WHERE playlist_id = $1 AND comparisons > 0 \
         ORDER BY comparisons DESC, wins DESC, title ASC LIMIT 1",
    )
    .bind(playlist.id)
    .fetch_optional(pool)
    .await?
    .map(|row| DuelledStory {
        comparisons: row.comparisons,
        track: row.into_track(),
    });

    let top_voter = sqlx::query_as::<_, TopVoterRow>(
        "SELECT u.username, COUNT(v.id) AS picks \
         FROM votes v JOIN users u ON u.id = v.user_id \
         WHERE v.playlist_id = $1 \
         GROUP BY u.id, u.username \
         ORDER BY picks DESC, MAX(v.created_at) DESC \
         LIMIT 1",
    )
    .bind(playlist.id)
    .fetch_optional(pool)
    .await?;

    tracing::info!(
        playlist_id = %playlist.id,
        duelled = most_duelled.as_ref().map(|story| story.track.title.as_str()).unwrap_or("-"),
        top_voter = top_voter.as_ref().map(|story| story.username.as_str()).unwrap_or("-"),
        "chart stories computed"
    );

    Ok(ChartStories {
        playlist_id: playlist.id,
        playlist_name: playlist.name.clone(),
        revealed: playlist.reveal_state == "revealed",
        most_duelled,
        top_voter: top_voter.map(|row| TopVoterStory {
            username: row.username,
            picks: row.picks,
        }),
    })
}
