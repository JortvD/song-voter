use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    /// Never serialised to responses.
    #[serde(skip)]
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    /// Disabled accounts may not sign in or use existing sessions.
    #[serde(skip)]
    pub disabled_at: Option<DateTime<Utc>>,
    /// Incremented to revoke every session issued before the change.
    #[serde(skip)]
    pub session_version: i64,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Invite {
    pub id: Uuid,
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub used_by_username: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Playlist {
    pub id: Uuid,
    pub spotify_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_name: Option<String>,
    pub imported_by: Option<Uuid>,
    pub imported_at: DateTime<Utc>,
    pub reveal_state: String,
    pub is_active: bool,
    pub scheduled_start_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub voting_paused: bool,
    pub voting_paused_at: Option<DateTime<Utc>>,
    pub scheduled_reveal_at: Option<DateTime<Utc>>,
    pub pause_voting_on_reveal: bool,
    pub start_error: Option<String>,
    pub start_attempts: i32,
    pub last_start_attempt_at: Option<DateTime<Utc>>,
    pub start_in_progress: bool,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Track {
    pub id: Uuid,
    pub playlist_id: Uuid,
    pub spotify_id: String,
    pub title: String,
    pub artists: String,
    pub album: Option<String>,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub duration_ms: Option<i32>,
    pub release_date: Option<String>,
    pub release_date_precision: Option<String>,
    pub release_year: Option<i32>,
    pub decade: Option<i32>,
    pub album_id: Option<String>,
    pub album_type: Option<String>,
    pub total_tracks: Option<i32>,
    pub disc_number: Option<i32>,
    pub track_number: Option<i32>,
    pub primary_genre: Option<String>,
    pub position: i32,
    pub rating: f64,
    pub comparisons: i32,
    pub wins: i32,
}

/// Public view of a track: no rating, comparisons or wins, so the ranking
/// cannot be inferred from duel responses.
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MatchupTrack {
    pub id: Uuid,
    pub spotify_id: String,
    pub title: String,
    pub artists: String,
    pub album: Option<String>,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub duration_ms: Option<i32>,
    pub release_year: Option<i32>,
    pub decade: Option<i32>,
    pub album_id: Option<String>,
    pub album_type: Option<String>,
    pub total_tracks: Option<i32>,
    pub disc_number: Option<i32>,
    pub track_number: Option<i32>,
    pub primary_genre: Option<String>,
}
