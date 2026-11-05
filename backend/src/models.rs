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
}
