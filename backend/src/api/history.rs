use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

/// Enough to fill the recent-duel gap on the largest phones.
pub const RECENT_DUEL_LIMIT: i64 = 8;
const MAX_PAGE_SIZE: i64 = 100;

#[derive(Debug, Clone, Serialize)]
pub struct DuelHistoryItem {
    /// Side A as originally served. Legacy rows fall back to the normalized
    /// low-id track because their orientation was not stored.
    pub track_a: MatchupTrack,
    /// Side B as originally served. Legacy rows fall back to the normalized
    /// high-id track.
    pub track_b: MatchupTrack,
    pub picked_track_id: Option<Uuid>,
    pub seen_at: DateTime<Utc>,
    pub picked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct DuelHistoryResponse {
    playlist_id: Uuid,
    playlist_name: String,
    total: i64,
    picked: i64,
    skipped: i64,
    filtered_total: i64,
    limit: i64,
    offset: i64,
    duels: Vec<DuelHistoryItem>,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HistoryFilter {
    All,
    Picked,
    Skipped,
}
impl HistoryFilter {
    fn from_query(value: Option<&str>) -> Self {
        match value.unwrap_or("all") {
            "picked" => Self::Picked,
            "skipped" => Self::Skipped,
            _ => Self::All,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct HistoryRow {
    track_low: Uuid,
    track_high: Uuid,
    served_flip: bool,
    seen_at: DateTime<Utc>,
    winner: Option<Uuid>,
    picked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct HistoryCounts {
    total: i64,
    picked: i64,
}

/// Every duel served to this user in the active playlist, including skipped
/// ones. Paginated and optionally filtered.
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<DuelHistoryResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let limit = query.limit.unwrap_or(50).clamp(1, MAX_PAGE_SIZE);
    let offset = query.offset.unwrap_or(0).max(0);
    let filter = HistoryFilter::from_query(query.status.as_deref());

    let mut connection = state.db.acquire().await?;
    let counts = history_counts(&mut connection, user.0.id, playlist.id).await?;
    let duels = fetch_history(
        &mut connection,
        user.0.id,
        playlist.id,
        filter,
        limit,
        offset,
    )
    .await?;

    let filtered_total = match filter {
        HistoryFilter::All => counts.total,
        HistoryFilter::Picked => counts.picked,
        HistoryFilter::Skipped => counts.total - counts.picked,
    };

    Ok(Json(DuelHistoryResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        total: counts.total,
        picked: counts.picked,
        skipped: counts.total - counts.picked,
        filtered_total,
        limit,
        offset,
        duels,
    }))
}

/// The recent list shown inline on the duel screen. Picked duels only, so the
/// current unpicked pair is naturally excluded.
pub async fn recent_picked(
    connection: &mut PgConnection,
    user_id: Uuid,
    playlist_id: Uuid,
    limit: i64,
) -> Result<Vec<DuelHistoryItem>, ApiError> {
    fetch_history(
        connection,
        user_id,
        playlist_id,
        HistoryFilter::Picked,
        limit,
        0,
    )
    .await
}

async fn history_counts(
    connection: &mut PgConnection,
    user_id: Uuid,
    playlist_id: Uuid,
) -> Result<HistoryCounts, ApiError> {
    let counts = sqlx::query_as::<_, HistoryCounts>(
        "SELECT COUNT(*) AS total, COUNT(v.winner) AS picked \
         FROM seen_pairs sp \
         LEFT JOIN votes v \
           ON v.user_id = sp.user_id \
          AND v.playlist_id = sp.playlist_id \
          AND v.track_low = sp.track_low \
          AND v.track_high = sp.track_high \
         WHERE sp.user_id = $1 AND sp.playlist_id = $2",
    )
    .bind(user_id)
    .bind(playlist_id)
    .fetch_one(&mut *connection)
    .await?;

    Ok(counts)
}

async fn fetch_history(
    connection: &mut PgConnection,
    user_id: Uuid,
    playlist_id: Uuid,
    filter: HistoryFilter,
    limit: i64,
    offset: i64,
) -> Result<Vec<DuelHistoryItem>, ApiError> {
    let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT sp.track_low, sp.track_high, sp.served_flip, sp.seen_at, v.winner, \
                v.created_at AS picked_at \
         FROM seen_pairs sp \
         LEFT JOIN votes v \
           ON v.user_id = sp.user_id \
          AND v.playlist_id = sp.playlist_id \
          AND v.track_low = sp.track_low \
          AND v.track_high = sp.track_high \
         WHERE sp.user_id = ",
    );
    builder.push_bind(user_id);
    builder
        .push(" AND sp.playlist_id = ")
        .push_bind(playlist_id);

    match filter {
        HistoryFilter::All => {}
        HistoryFilter::Picked => {
            builder.push(" AND v.winner IS NOT NULL");
        }
        HistoryFilter::Skipped => {
            builder.push(" AND v.winner IS NULL");
        }
    }

    builder
        .push(" ORDER BY sp.seen_at DESC, sp.track_low, sp.track_high LIMIT ")
        .push_bind(limit);

    if offset > 0 {
        builder.push(" OFFSET ").push_bind(offset);
    }

    let rows = builder
        .build_query_as::<HistoryRow>()
        .fetch_all(&mut *connection)
        .await?;

    hydrate_history(connection, rows).await
}

async fn hydrate_history(
    connection: &mut PgConnection,
    rows: Vec<HistoryRow>,
) -> Result<Vec<DuelHistoryItem>, ApiError> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let ids: Vec<Uuid> = rows
        .iter()
        .flat_map(|row| [row.track_low, row.track_high])
        .collect();

    let tracks = sqlx::query_as::<_, MatchupTrack>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, release_year, decade, album_id, album_type, total_tracks, \
                disc_number, track_number, primary_genre \
         FROM tracks WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&mut *connection)
    .await?;

    let by_id: HashMap<Uuid, MatchupTrack> =
        tracks.into_iter().map(|track| (track.id, track)).collect();

    rows.into_iter()
        .map(|row| {
            let low = by_id.get(&row.track_low).cloned().ok_or_else(|| {
                ApiError::internal(anyhow::anyhow!("history low track is missing"))
            })?;
            let high = by_id.get(&row.track_high).cloned().ok_or_else(|| {
                ApiError::internal(anyhow::anyhow!("history high track is missing"))
            })?;

            let (track_a, track_b) = if row.served_flip {
                (high, low)
            } else {
                (low, high)
            };

            Ok(DuelHistoryItem {
                track_a,
                track_b,
                picked_track_id: row.winner,
                seen_at: row.seen_at,
                picked_at: row.picked_at,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_filter_defaults_to_all() {
        assert_eq!(HistoryFilter::from_query(None), HistoryFilter::All);
    }

    #[test]
    fn history_filter_parses_picked_and_skipped() {
        assert_eq!(
            HistoryFilter::from_query(Some("picked")),
            HistoryFilter::Picked
        );
        assert_eq!(
            HistoryFilter::from_query(Some("skipped")),
            HistoryFilter::Skipped
        );
    }

    #[test]
    fn history_filter_accepts_all_as_a_value() {
        assert_eq!(HistoryFilter::from_query(Some("all")), HistoryFilter::All);
    }

    #[test]
    fn history_filter_treats_unknown_values_as_all() {
        for value in ["", "PICKED", "done", "skipped "] {
            assert_eq!(
                HistoryFilter::from_query(Some(value)),
                HistoryFilter::All,
                "{value:?}"
            );
        }
    }

    #[test]
    fn filtered_totals_are_derived_from_the_counts() {
        let counts = HistoryCounts {
            total: 10,
            picked: 7,
        };
        assert_eq!(counts.total, 10);
        assert_eq!(counts.picked, 7);
        assert_eq!(counts.total - counts.picked, 3);
    }
}
