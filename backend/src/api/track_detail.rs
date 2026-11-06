//! One song's full dossier for the standings and Your Ranking.
//!
//! Public callers only see the official numbers after The Reveal. Signed-in
//! voters also get their personal record, which is safe to show while the
//! group standings are still sealed.

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::ranking::personal_track_snapshot;
use crate::auth::OptionalAuthUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

const RECENT_DUEL_LIMIT: i64 = 6;

#[derive(Debug, Serialize)]
pub struct TrackDetailResponse {
    playlist_id: Uuid,
    playlist_name: String,
    revealed: bool,
    track: MatchupTrack,
    album: Option<AlbumDetail>,
    artists: Vec<ArtistDetail>,
    official: Option<OfficialTrackStats>,
    personal: Option<PersonalTrackStats>,
}

#[derive(Debug, Serialize)]
pub struct OfficialTrackStats {
    rank: i64,
    total_tracks: i64,
    rating: f64,
    total_rating_average: Option<f64>,
    comparisons: i64,
    wins: i64,
    losses: i64,
    win_rate: Option<f64>,
    voters: i64,
    unique_opponents: i64,
    first_duel_at: Option<DateTime<Utc>>,
    last_duel_at: Option<DateTime<Utc>>,
    recent_duels: Vec<TrackDuel>,
}

#[derive(Debug, Serialize)]
pub struct PersonalTrackStats {
    rank: Option<usize>,
    total_ranked: usize,
    rating: Option<f64>,
    comparisons: i64,
    wins: i64,
    losses: i64,
    win_rate: Option<f64>,
    first_duel_at: Option<DateTime<Utc>>,
    last_duel_at: Option<DateTime<Utc>>,
    recent_duels: Vec<TrackDuel>,
}

#[derive(Debug, Serialize)]
pub struct AlbumDetail {
    id: Option<String>,
    name: String,
    artists: String,
    image_url: Option<String>,
    album_type: Option<String>,
    release_year: Option<i32>,
    total_tracks: Option<i32>,
    playlist_track_count: i64,
    average_rating: Option<f64>,
    best_track: Option<TrackHighlight>,
}

#[derive(Debug, Serialize)]
pub struct ArtistDetail {
    id: Uuid,
    name: String,
    genre: Option<String>,
    track_count: i64,
    average_rating: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrackHighlight {
    id: Uuid,
    spotify_id: String,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct TrackDuel {
    opponent: TrackHighlight,
    won: bool,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct OfficialRow {
    official_rank: i64,
    total_tracks: i64,
    rating: f64,
    comparisons: i32,
    wins: i32,
    total_rating_average: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct OfficialVoteStats {
    voters: i64,
    unique_opponents: i64,
    first_duel_at: Option<DateTime<Utc>>,
    last_duel_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct PersonalVoteStats {
    comparisons: i64,
    wins: i64,
    first_duel_at: Option<DateTime<Utc>>,
    last_duel_at: Option<DateTime<Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct RecentDuelRow {
    opponent_id: Uuid,
    opponent_spotify_id: String,
    opponent_title: String,
    opponent_artists: String,
    opponent_image_url: Option<String>,
    won: bool,
    occurred_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct AlbumCounts {
    playlist_track_count: i64,
    average_rating: Option<f64>,
}

#[derive(Debug, sqlx::FromRow)]
struct AlbumBestRow {
    id: Uuid,
    spotify_id: String,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct ArtistRow {
    id: Uuid,
    name: String,
    genre: Option<String>,
    track_count: i64,
    average_rating: Option<f64>,
}

pub async fn detail(
    State(state): State<AppState>,
    OptionalAuthUser(user): OptionalAuthUser,
    Path(track_id): Path<Uuid>,
) -> Result<Json<TrackDetailResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let track = sqlx::query_as::<_, MatchupTrack>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, release_year, decade, album_id, album_type, total_tracks, \
                disc_number, track_number, primary_genre \
         FROM tracks WHERE id = $1 AND playlist_id = $2",
    )
    .bind(track_id)
    .bind(playlist.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("song not found in the active playlist".to_string()))?;

    let revealed = playlist.reveal_state == "revealed";
    if !revealed && user.is_none() {
        return Err(ApiError::Forbidden(
            "the standings have not been revealed yet".to_string(),
        ));
    }

    let album = load_album(&state.db, playlist.id, &track, revealed).await?;
    let artists = load_artists(&state.db, playlist.id, track_id, revealed).await?;

    let official = if revealed {
        Some(load_official(&state.db, playlist.id, track_id).await?)
    } else {
        None
    };

    let personal = match &user {
        Some(user) => Some(load_personal(&state.db, user.id, playlist.id, track_id).await?),
        None => None,
    };

    Ok(Json(TrackDetailResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        revealed,
        track,
        album,
        artists,
        official,
        personal,
    }))
}

async fn load_official(
    pool: &PgPool,
    playlist_id: Uuid,
    track_id: Uuid,
) -> Result<OfficialTrackStats, ApiError> {
    let row = sqlx::query_as::<_, OfficialRow>(
        "WITH ranked AS ( \
             SELECT id, rating, comparisons, wins, \
                    ROW_NUMBER() OVER ( \
                        ORDER BY rating DESC, wins DESC, title ASC, id ASC \
                    ) AS official_rank, \
                    COUNT(*) OVER () AS total_tracks \
             FROM tracks WHERE playlist_id = $1 \
         ) \
         SELECT r.official_rank, r.total_tracks, r.rating, r.comparisons, r.wins, \
                (SELECT AVG(t2.rating)::float8 FROM tracks t2 \
                 WHERE t2.playlist_id = $1 AND t2.comparisons > 0) AS total_rating_average \
         FROM ranked r WHERE r.id = $2",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_one(pool)
    .await?;

    let votes = sqlx::query_as::<_, OfficialVoteStats>(
        "SELECT COUNT(DISTINCT user_id)::bigint AS voters, \
                COUNT(DISTINCT CASE WHEN track_low = $2 THEN track_high ELSE track_low END)::bigint \
                    AS unique_opponents, \
                MIN(created_at) AS first_duel_at, \
                MAX(created_at) AS last_duel_at \
         FROM votes \
         WHERE playlist_id = $1 AND (track_low = $2 OR track_high = $2)",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_one(pool)
    .await?;

    let recent_duels = load_recent_duels(pool, playlist_id, track_id, None).await?;
    let comparisons = i64::from(row.comparisons);
    let wins = i64::from(row.wins);
    let losses = (comparisons - wins).max(0);

    Ok(OfficialTrackStats {
        rank: row.official_rank,
        total_tracks: row.total_tracks,
        rating: row.rating,
        total_rating_average: row.total_rating_average,
        comparisons,
        wins,
        losses,
        win_rate: ratio(wins, comparisons),
        voters: votes.voters,
        unique_opponents: votes.unique_opponents,
        first_duel_at: votes.first_duel_at,
        last_duel_at: votes.last_duel_at,
        recent_duels,
    })
}

async fn load_personal(
    pool: &PgPool,
    user_id: Uuid,
    playlist_id: Uuid,
    track_id: Uuid,
) -> Result<PersonalTrackStats, ApiError> {
    let snapshot = personal_track_snapshot(pool, user_id, playlist_id, track_id).await?;

    let votes = sqlx::query_as::<_, PersonalVoteStats>(
        "SELECT COUNT(*)::bigint AS comparisons, \
                COUNT(*) FILTER (WHERE winner = $3)::bigint AS wins, \
                MIN(created_at) AS first_duel_at, \
                MAX(created_at) AS last_duel_at \
         FROM votes \
         WHERE user_id = $1 AND playlist_id = $2 \
           AND (track_low = $3 OR track_high = $3)",
    )
    .bind(user_id)
    .bind(playlist_id)
    .bind(track_id)
    .fetch_one(pool)
    .await?;

    let recent_duels = load_recent_duels(pool, playlist_id, track_id, Some(user_id)).await?;
    let losses = (votes.comparisons - votes.wins).max(0);

    Ok(PersonalTrackStats {
        rank: snapshot.rank,
        total_ranked: snapshot.total_ranked,
        rating: snapshot.rating,
        comparisons: votes.comparisons,
        wins: votes.wins,
        losses,
        win_rate: ratio(votes.wins, votes.comparisons),
        first_duel_at: votes.first_duel_at,
        last_duel_at: votes.last_duel_at,
        recent_duels,
    })
}

async fn load_recent_duels(
    pool: &PgPool,
    playlist_id: Uuid,
    track_id: Uuid,
    user_id: Option<Uuid>,
) -> Result<Vec<TrackDuel>, ApiError> {
    let rows = match user_id {
        Some(user_id) => {
            sqlx::query_as::<_, RecentDuelRow>(
                "SELECT CASE WHEN v.track_low = $3 THEN v.track_high ELSE v.track_low END \
                            AS opponent_id, \
                        t.spotify_id AS opponent_spotify_id, \
                        t.title AS opponent_title, \
                        t.artists AS opponent_artists, \
                        t.album_image_url AS opponent_image_url, \
                        (v.winner = $3) AS won, \
                        v.created_at AS occurred_at \
                 FROM votes v \
                 JOIN tracks t ON t.id = CASE WHEN v.track_low = $3 \
                                              THEN v.track_high ELSE v.track_low END \
                 WHERE v.user_id = $1 AND v.playlist_id = $2 \
                   AND (v.track_low = $3 OR v.track_high = $3) \
                 ORDER BY v.created_at DESC \
                 LIMIT $4",
            )
            .bind(user_id)
            .bind(playlist_id)
            .bind(track_id)
            .bind(RECENT_DUEL_LIMIT)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, RecentDuelRow>(
                "SELECT CASE WHEN v.track_low = $2 THEN v.track_high ELSE v.track_low END \
                            AS opponent_id, \
                        t.spotify_id AS opponent_spotify_id, \
                        t.title AS opponent_title, \
                        t.artists AS opponent_artists, \
                        t.album_image_url AS opponent_image_url, \
                        (v.winner = $2) AS won, \
                        v.created_at AS occurred_at \
                 FROM votes v \
                 JOIN tracks t ON t.id = CASE WHEN v.track_low = $2 \
                                              THEN v.track_high ELSE v.track_low END \
                 WHERE v.playlist_id = $1 \
                   AND (v.track_low = $2 OR v.track_high = $2) \
                 ORDER BY v.created_at DESC \
                 LIMIT $3",
            )
            .bind(playlist_id)
            .bind(track_id)
            .bind(RECENT_DUEL_LIMIT)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows
        .into_iter()
        .map(|row| TrackDuel {
            opponent: TrackHighlight {
                id: row.opponent_id,
                spotify_id: row.opponent_spotify_id,
                title: row.opponent_title,
                artists: row.opponent_artists,
                album_image_url: row.opponent_image_url,
                rating: None,
            },
            won: row.won,
            occurred_at: row.occurred_at,
        })
        .collect())
}

async fn load_album(
    pool: &PgPool,
    playlist_id: Uuid,
    track: &MatchupTrack,
    revealed: bool,
) -> Result<Option<AlbumDetail>, ApiError> {
    if track.album.is_none() && track.album_id.is_none() {
        return Ok(None);
    }

    let album_id = track.album_id.as_deref();
    let album_name = track.album.as_deref();

    let counts = sqlx::query_as::<_, AlbumCounts>(
        "SELECT COUNT(*)::bigint AS playlist_track_count, \
                AVG(rating) FILTER (WHERE comparisons > 0 AND $4::bool)::float8 \
                    AS average_rating \
         FROM tracks \
         WHERE playlist_id = $1 \
           AND (($2::text IS NOT NULL AND album_id = $2) \
                OR ($2::text IS NULL AND $3::text IS NOT NULL AND album = $3))",
    )
    .bind(playlist_id)
    .bind(album_id)
    .bind(album_name)
    .bind(revealed)
    .fetch_one(pool)
    .await?;

    let best_track = if revealed {
        sqlx::query_as::<_, AlbumBestRow>(
            "SELECT id, spotify_id, title, artists, album_image_url, rating \
             FROM tracks \
             WHERE playlist_id = $1 AND comparisons > 0 \
               AND (($2::text IS NOT NULL AND album_id = $2) \
                    OR ($2::text IS NULL AND $3::text IS NOT NULL AND album = $3)) \
             ORDER BY rating DESC, wins DESC, title ASC, id ASC \
             LIMIT 1",
        )
        .bind(playlist_id)
        .bind(album_id)
        .bind(album_name)
        .fetch_optional(pool)
        .await?
        .map(|row| TrackHighlight {
            id: row.id,
            spotify_id: row.spotify_id,
            title: row.title,
            artists: row.artists,
            album_image_url: row.album_image_url,
            rating: Some(row.rating),
        })
    } else {
        None
    };

    Ok(Some(AlbumDetail {
        id: track.album_id.clone(),
        name: track
            .album
            .clone()
            .unwrap_or_else(|| "Unknown album".to_string()),
        artists: track.artists.clone(),
        image_url: track.album_image_url.clone(),
        album_type: track.album_type.clone(),
        release_year: track.release_year,
        total_tracks: track.total_tracks,
        playlist_track_count: counts.playlist_track_count,
        average_rating: counts.average_rating,
        best_track,
    }))
}

async fn load_artists(
    pool: &PgPool,
    playlist_id: Uuid,
    track_id: Uuid,
    revealed: bool,
) -> Result<Vec<ArtistDetail>, ApiError> {
    let rows = sqlx::query_as::<_, ArtistRow>(
        "SELECT a.id, a.name, a.primary_genre AS genre, \
                COUNT(DISTINCT t.id)::bigint AS track_count, \
                AVG(t.rating) FILTER (WHERE t.comparisons > 0)::float8 AS average_rating \
         FROM track_artists target \
         JOIN artists a ON a.id = target.artist_id \
         LEFT JOIN track_artists ta ON ta.artist_id = a.id \
         LEFT JOIN tracks t ON t.id = ta.track_id AND t.playlist_id = $1 \
         WHERE target.track_id = $2 \
         GROUP BY a.id, a.name, a.primary_genre, target.position \
         ORDER BY target.position ASC",
    )
    .bind(playlist_id)
    .bind(track_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ArtistDetail {
            id: row.id,
            name: row.name,
            genre: row.genre,
            track_count: row.track_count,
            average_rating: if revealed { row.average_rating } else { None },
        })
        .collect())
}

fn ratio(numerator: i64, denominator: i64) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

#[cfg(test)]
mod tests {
    use super::ratio;

    #[test]
    fn ratios_are_none_without_duels() {
        assert_eq!(ratio(0, 0), None);
    }

    #[test]
    fn ratios_are_fractions_of_the_duel_count() {
        assert_eq!(ratio(3, 4), Some(0.75));
    }
}
