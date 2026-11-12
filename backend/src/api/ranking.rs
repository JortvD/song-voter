use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::streak::{load_streak, StreakQuery, StreakSummary};
use crate::auth::AuthUser;
use crate::db::active_playlist;
use crate::elo;
use crate::error::ApiError;
use crate::models::MatchupTrack;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RankingQuery {
    limit: Option<i64>,
    offset: Option<i64>,
    q: Option<String>,
    decade: Option<i32>,
    genre: Option<String>,
    artist: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct PersonalRankingResponse {
    playlist_id: Uuid,
    playlist_name: String,
    /// Number of songs matching the current filters.
    total: usize,
    /// Number of songs in the voter's unfiltered personal ranking.
    ranked_total: usize,
    top_score: Option<f64>,
    votes: usize,
    limit: i64,
    offset: i64,
    decades: Vec<PersonalFacet>,
    genres: Vec<PersonalFacet>,
    artists: Vec<PersonalArtistFacet>,
    tracks: Vec<RankedTrack>,
}

#[derive(Debug, Serialize)]
pub struct PersonalFacet {
    value: String,
    count: i64,
    average_rating: Option<f64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PersonalArtistFacet {
    id: String,
    name: String,
    count: i64,
}

#[derive(Debug, Serialize)]
pub struct RankedTrack {
    rank: usize,
    track: MatchupTrack,
    rating: f64,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, Serialize)]
pub struct PersonalSummaryResponse {
    playlist_id: Uuid,
    playlist_name: String,
    votes: usize,
    total: usize,
    top_track: Option<PersonalSummaryTrack>,
    streak: StreakSummary,
}

#[derive(Debug, Serialize)]
pub struct PersonalSummaryTrack {
    id: Uuid,
    title: String,
    artists: String,
    album_image_url: Option<String>,
    rating: f64,
    comparisons: i32,
    wins: i32,
}

#[derive(Debug, sqlx::FromRow)]
struct PersonalVote {
    track_low: Uuid,
    track_high: Uuid,
    winner: Uuid,
}

/// Replays only this user's votes to produce their personal Elo chart.
/// The global ranking stays hidden until the reveal.
pub async fn personal(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<RankingQuery>,
) -> Result<Json<PersonalRankingResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let search = query.q.unwrap_or_default().trim().to_string();
    crate::error::validate_text_input(&search, "search")?;
    let search_lower = search.to_lowercase();
    let genre = query
        .genre
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let Some(genre) = genre.as_deref() {
        crate::error::validate_text_input(genre, "genre")?;
    }
    let started = Instant::now();

    let (by_id, votes) = user_tracks_and_votes(&state.db, user.0.id, playlist.id).await?;
    let ranked = replay_tracks(by_id, &votes);
    let ranked_total = ranked.len();
    let top_score = ranked.first().map(|entry| entry.rating);

    // Facets describe only songs this voter has actually ranked, so the filter
    // panel cannot offer artists or eras that would always return nothing.
    let decades = build_decade_facets(&ranked);
    let genres = build_genre_facets(&ranked);
    let artists = load_artist_facets(&state.db, &ranked).await?;

    let artist_track_ids = match query.artist {
        Some(artist_id) => Some(load_artist_track_ids(&state.db, playlist.id, artist_id).await?),
        None => None,
    };

    let ranked: Vec<RankedTrack> = ranked
        .into_iter()
        .filter(|entry| {
            let track = &entry.track;

            if !search_lower.is_empty()
                && !track.title.to_lowercase().contains(&search_lower)
                && !track.artists.to_lowercase().contains(&search_lower)
                && !track
                    .album
                    .as_deref()
                    .is_some_and(|album| album.to_lowercase().contains(&search_lower))
            {
                return false;
            }

            if query
                .decade
                .is_some_and(|decade| track.decade != Some(decade))
            {
                return false;
            }

            if let Some(genre) = genre.as_deref() {
                if track.primary_genre.as_deref() != Some(genre) {
                    return false;
                }
            }

            if let Some(ids) = &artist_track_ids {
                if !ids.contains(&track.id) {
                    return false;
                }
            }

            true
        })
        .collect();

    let total = ranked.len();
    let page: Vec<RankedTrack> = ranked
        .into_iter()
        .skip(usize::try_from(offset).unwrap_or(0))
        .take(usize::try_from(limit).unwrap_or(50))
        .collect();

    tracing::debug!(
        user_id = %user.0.id,
        playlist_id = %playlist.id,
        votes = votes.len(),
        ranked_tracks = total,
        elapsed_ms = started.elapsed().as_millis() as u64,
        "personal ranking computed"
    );

    Ok(Json(PersonalRankingResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        total,
        ranked_total,
        top_score,
        votes: votes.len(),
        limit,
        offset,
        decades,
        genres,
        artists,
        tracks: page,
    }))
}

fn build_decade_facets(ranked: &[RankedTrack]) -> Vec<PersonalFacet> {
    let mut groups: HashMap<i32, (i64, f64)> = HashMap::new();

    for entry in ranked {
        let Some(decade) = entry.track.decade else {
            continue;
        };
        let group = groups.entry(decade).or_insert((0, 0.0));
        group.0 += 1;
        group.1 += entry.rating;
    }

    let mut facets: Vec<PersonalFacet> = groups
        .into_iter()
        .map(|(value, (count, total))| PersonalFacet {
            value: value.to_string(),
            count,
            average_rating: Some(total / count as f64),
        })
        .collect();
    facets.sort_by_key(|facet| facet.value.parse::<i32>().unwrap_or_default());
    facets
}

fn build_genre_facets(ranked: &[RankedTrack]) -> Vec<PersonalFacet> {
    let mut groups: HashMap<String, (i64, f64)> = HashMap::new();

    for entry in ranked {
        let Some(genre) = entry.track.primary_genre.clone() else {
            continue;
        };
        let group = groups.entry(genre).or_insert((0, 0.0));
        group.0 += 1;
        group.1 += entry.rating;
    }

    let mut facets: Vec<PersonalFacet> = groups
        .into_iter()
        .map(|(value, (count, total))| PersonalFacet {
            value,
            count,
            average_rating: Some(total / count as f64),
        })
        .collect();
    facets.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.value.cmp(&right.value))
    });
    facets
}

async fn load_artist_facets(
    pool: &PgPool,
    ranked: &[RankedTrack],
) -> Result<Vec<PersonalArtistFacet>, ApiError> {
    let track_ids: Vec<Uuid> = ranked.iter().map(|entry| entry.track.id).collect();
    if track_ids.is_empty() {
        return Ok(Vec::new());
    }

    let facets = sqlx::query_as::<_, PersonalArtistFacet>(
        "SELECT a.id::text AS id, a.name, COUNT(DISTINCT ta.track_id)::bigint AS count \
         FROM track_artists ta \
         JOIN artists a ON a.id = ta.artist_id \
         WHERE ta.track_id = ANY($1) \
         GROUP BY a.id, a.name \
         ORDER BY a.name ASC",
    )
    .bind(track_ids)
    .fetch_all(pool)
    .await?;

    Ok(facets)
}

async fn load_artist_track_ids(
    pool: &PgPool,
    playlist_id: Uuid,
    artist_id: Uuid,
) -> Result<HashSet<Uuid>, ApiError> {
    let ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT ta.track_id \
         FROM track_artists ta \
         JOIN tracks t ON t.id = ta.track_id \
         WHERE ta.artist_id = $1 AND t.playlist_id = $2",
    )
    .bind(artist_id)
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;

    Ok(ids.into_iter().collect())
}

/// Lightweight version of the personal ranking for the home page. It avoids
/// transferring a full ranking page when the UI only needs the counters and #1.
pub async fn summary(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<StreakQuery>,
) -> Result<Json<PersonalSummaryResponse>, ApiError> {
    let Some(playlist) = active_playlist(&state.db).await? else {
        return Err(ApiError::NotFound(
            "no active playlist has been imported yet".to_string(),
        ));
    };

    let (by_id, votes) = user_tracks_and_votes(&state.db, user.0.id, playlist.id).await?;
    let ranked = replay_tracks(by_id, &votes);

    let top_track = ranked.first().map(|entry| PersonalSummaryTrack {
        id: entry.track.id,
        title: entry.track.title.clone(),
        artists: entry.track.artists.clone(),
        album_image_url: entry.track.album_image_url.clone(),
        rating: entry.rating,
        comparisons: entry.comparisons,
        wins: entry.wins,
    });

    let mut connection = state.db.acquire().await?;
    let streak = load_streak(&mut connection, user.0.id, query.tz.as_deref()).await?;

    Ok(Json(PersonalSummaryResponse {
        playlist_id: playlist.id,
        playlist_name: playlist.name,
        votes: votes.len(),
        total: ranked.len(),
        top_track,
        streak,
    }))
}

/// Load only tracks this user has actually voted on. Replaying a 3,000-song
/// playlist from scratch is wasted work when a voter has seen a few hundred.
async fn user_tracks_and_votes(
    pool: &PgPool,
    user_id: Uuid,
    playlist_id: Uuid,
) -> Result<(HashMap<Uuid, MatchupTrack>, Vec<PersonalVote>), ApiError> {
    let tracks = sqlx::query_as::<_, MatchupTrack>(
        "WITH user_tracks AS ( \
             SELECT track_low AS id FROM votes \
             WHERE user_id = $1 AND playlist_id = $2 \
             UNION \
             SELECT track_high FROM votes \
             WHERE user_id = $1 AND playlist_id = $2 \
         ) \
         SELECT t.id, t.spotify_id, t.title, t.artists, t.album, t.album_image_url, \
                t.preview_url, t.duration_ms, t.release_year, t.decade, t.album_id, \
                t.album_type, t.total_tracks, t.disc_number, t.track_number, t.primary_genre \
         FROM tracks t JOIN user_tracks ut ON ut.id = t.id \
         ORDER BY t.id",
    )
    .bind(user_id)
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;

    let votes = sqlx::query_as::<_, PersonalVote>(
        "SELECT track_low, track_high, winner FROM votes \
         WHERE user_id = $1 AND playlist_id = $2 \
         ORDER BY seq ASC",
    )
    .bind(user_id)
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;

    let by_id: HashMap<Uuid, MatchupTrack> =
        tracks.into_iter().map(|track| (track.id, track)).collect();

    Ok((by_id, votes))
}

pub(crate) struct PersonalTrackSnapshot {
    pub rank: Option<usize>,
    pub total_ranked: usize,
    pub rating: Option<f64>,
}

/// Replay this voter's picks and pluck one track out of the personal chart.
/// Used by the song detail dialog so its numbers always match Your Ranking.
pub(crate) async fn personal_track_snapshot(
    pool: &PgPool,
    user_id: Uuid,
    playlist_id: Uuid,
    track_id: Uuid,
) -> Result<PersonalTrackSnapshot, ApiError> {
    let (by_id, votes) = user_tracks_and_votes(pool, user_id, playlist_id).await?;
    let ranked = replay_tracks(by_id, &votes);
    let total_ranked = ranked.len();

    let found = ranked.into_iter().find(|entry| entry.track.id == track_id);

    Ok(match found {
        Some(entry) => PersonalTrackSnapshot {
            rank: Some(entry.rank),
            total_ranked,
            rating: Some(entry.rating),
        },
        None => PersonalTrackSnapshot {
            rank: None,
            total_ranked,
            rating: None,
        },
    })
}

fn replay_tracks(by_id: HashMap<Uuid, MatchupTrack>, votes: &[PersonalVote]) -> Vec<RankedTrack> {
    let mut ratings: HashMap<Uuid, f64> =
        by_id.keys().map(|id| (*id, elo::INITIAL_RATING)).collect();
    let mut comparisons: HashMap<Uuid, i32> = by_id.keys().map(|id| (*id, 0)).collect();
    let mut wins: HashMap<Uuid, i32> = by_id.keys().map(|id| (*id, 0)).collect();

    for vote in votes {
        if !by_id.contains_key(&vote.track_low) || !by_id.contains_key(&vote.track_high) {
            continue;
        }

        let low_won = vote.winner == vote.track_low;
        let outcome = elo::update(
            ratings[&vote.track_low],
            ratings[&vote.track_high],
            comparisons[&vote.track_low],
            comparisons[&vote.track_high],
            low_won,
        );

        *ratings.get_mut(&vote.track_low).expect("known track") += outcome.delta_low;
        *ratings.get_mut(&vote.track_high).expect("known track") += outcome.delta_high;
        *comparisons.get_mut(&vote.track_low).expect("known track") += 1;
        *comparisons.get_mut(&vote.track_high).expect("known track") += 1;
        if let Some(wins) = wins.get_mut(&vote.winner) {
            *wins += 1;
        }
    }

    let mut ranked: Vec<RankedTrack> = by_id
        .into_values()
        .filter_map(|track| {
            let track_comparisons = comparisons[&track.id];
            if track_comparisons == 0 {
                return None;
            }

            Some(RankedTrack {
                rank: 0,
                rating: ratings[&track.id],
                comparisons: track_comparisons,
                wins: wins[&track.id],
                track,
            })
        })
        .collect();

    ranked.sort_by(|left, right| {
        right
            .rating
            .partial_cmp(&left.rating)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.wins.cmp(&left.wins))
            .then_with(|| left.track.title.cmp(&right.track.title))
    });

    for (index, entry) in ranked.iter_mut().enumerate() {
        entry.rank = index + 1;
    }

    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: u128, title: &str, decade: Option<i32>, genre: Option<&str>) -> MatchupTrack {
        MatchupTrack {
            id: Uuid::from_u128(id),
            spotify_id: format!("spotify-{id}"),
            title: title.to_string(),
            artists: "Artist".to_string(),
            album: Some("Album".to_string()),
            album_image_url: None,
            preview_url: None,
            duration_ms: Some(180_000),
            release_year: decade.map(|value| value + 1),
            decade,
            album_id: None,
            album_type: None,
            total_tracks: None,
            disc_number: None,
            track_number: None,
            primary_genre: genre.map(str::to_string),
        }
    }

    fn vote(low: u128, high: u128, winner: u128) -> PersonalVote {
        PersonalVote {
            track_low: Uuid::from_u128(low),
            track_high: Uuid::from_u128(high),
            winner: Uuid::from_u128(winner),
        }
    }

    fn by_id(tracks: Vec<MatchupTrack>) -> HashMap<Uuid, MatchupTrack> {
        tracks.into_iter().map(|track| (track.id, track)).collect()
    }

    fn ranked(
        id: u128,
        title: &str,
        rating: f64,
        decade: Option<i32>,
        genre: Option<&str>,
    ) -> RankedTrack {
        RankedTrack {
            rank: 1,
            track: track(id, title, decade, genre),
            rating,
            comparisons: 2,
            wins: 1,
        }
    }

    #[test]
    fn personal_replay_ignores_tracks_without_votes() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn personal_replay_starts_at_one_thousand_and_counts_comparisons() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 2, 1)]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].rank, 1);
        assert_eq!(result[1].rank, 2);
        let winner = result
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(1))
            .unwrap();
        let loser = result
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(2))
            .unwrap();
        assert_eq!(winner.wins, 1);
        assert_eq!(loser.wins, 0);
        assert_eq!(winner.comparisons, 1);
        assert!(winner.rating > 1000.0);
        assert!(loser.rating < 1000.0);
    }

    #[test]
    fn personal_replay_orders_by_rating_then_wins_then_title() {
        let tracks = by_id(vec![
            track(1, "B", None, None),
            track(2, "A", None, None),
            track(3, "C", None, None),
        ]);
        let result = replay_tracks(tracks, &[vote(1, 2, 1), vote(1, 2, 2)]);
        // Both tracks have one win and one loss, so the title breaks the tie.
        assert_eq!(result[0].track.title, "A");
        assert_eq!(result[1].track.title, "B");
    }

    #[test]
    fn personal_replay_skips_votes_for_unknown_tracks() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 99, 1)]);
        assert!(result.is_empty());
    }

    #[test]
    fn personal_replay_does_not_panic_for_an_unknown_winner() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 2, 99)]);

        assert_eq!(result.len(), 2);
        assert_eq!(result.iter().map(|entry| entry.wins).sum::<i32>(), 0);
        assert_eq!(result.iter().map(|entry| entry.comparisons).sum::<i32>(), 2);
    }

    #[test]
    fn personal_replay_is_zero_sum_between_two_tracks() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 2, 2)]);
        let total = result.iter().map(|entry| entry.rating).sum::<f64>();
        assert!((total - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn personal_replay_handles_multiple_votes_and_repeated_pairs() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 2, 1), vote(1, 2, 2), vote(1, 2, 1)]);
        let a = result
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(1))
            .unwrap();
        let b = result
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(2))
            .unwrap();
        assert_eq!(a.wins, 2);
        assert_eq!(b.wins, 1);
        assert_eq!(a.comparisons, 3);
        assert_eq!(b.comparisons, 3);
    }

    #[test]
    fn personal_replay_only_needs_the_tracks_it_receives() {
        let tracks = by_id(vec![track(1, "A", None, None)]);
        let result = replay_tracks(tracks, &[vote(1, 2, 1)]);
        assert!(result.is_empty());
    }

    #[test]
    fn decade_facets_group_and_average_ratings() {
        let entries = vec![
            ranked(1, "A", 1000.0, Some(1990), None),
            ranked(2, "B", 1200.0, Some(1990), None),
            ranked(3, "C", 1100.0, Some(2000), None),
            ranked(4, "D", 1300.0, None, None),
        ];

        let facets = build_decade_facets(&entries);

        assert_eq!(facets.len(), 2);
        assert_eq!(facets[0].value, "1990");
        assert_eq!(facets[0].count, 2);
        assert_eq!(facets[0].average_rating, Some(1100.0));
        assert_eq!(facets[1].value, "2000");
        assert_eq!(facets[1].count, 1);
    }

    #[test]
    fn decade_facets_are_sorted_numerically() {
        let entries = vec![
            ranked(1, "A", 1000.0, Some(2020), None),
            ranked(2, "B", 1000.0, Some(1980), None),
            ranked(3, "C", 1000.0, Some(2000), None),
        ];
        let values = build_decade_facets(&entries)
            .into_iter()
            .map(|facet| facet.value)
            .collect::<Vec<_>>();
        assert_eq!(values, vec!["1980", "2000", "2020"]);
    }

    #[test]
    fn decade_facets_are_empty_without_decades() {
        assert!(build_decade_facets(&[]).is_empty());
        assert!(build_decade_facets(&[ranked(1, "A", 1000.0, None, None)]).is_empty());
    }

    #[test]
    fn genre_facets_group_and_sort_by_count_then_name() {
        let entries = vec![
            ranked(1, "A", 1000.0, None, Some("Rock")),
            ranked(2, "B", 1200.0, None, Some("Rock")),
            ranked(3, "C", 1100.0, None, Some("Pop")),
            ranked(4, "D", 1400.0, None, Some("Jazz")),
        ];

        let facets = build_genre_facets(&entries);

        assert_eq!(facets.len(), 3);
        assert_eq!(facets[0].value, "Rock");
        assert_eq!(facets[0].count, 2);
        assert_eq!(facets[0].average_rating, Some(1100.0));
        assert_eq!(facets[1].value, "Jazz");
        assert_eq!(facets[1].count, 1);
        assert_eq!(facets[2].value, "Pop");
    }

    #[test]
    fn genre_facets_are_empty_without_genres() {
        assert!(build_genre_facets(&[]).is_empty());
        assert!(build_genre_facets(&[ranked(1, "A", 1000.0, Some(1990), None)]).is_empty());
    }

    #[test]
    fn genre_facets_keep_distinct_names() {
        let entries = vec![
            ranked(1, "A", 1000.0, None, Some("Pop")),
            ranked(2, "B", 1000.0, None, Some("Rock")),
        ];
        assert_eq!(build_genre_facets(&entries).len(), 2);
    }

    #[test]
    fn rank_is_one_based_for_every_entry() {
        let tracks = by_id(vec![
            track(1, "A", None, None),
            track(2, "B", None, None),
            track(3, "C", None, None),
        ]);
        let result = replay_tracks(tracks, &[vote(1, 2, 1), vote(2, 3, 2)]);
        assert_eq!(
            result.iter().map(|entry| entry.rank).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    #[test]
    fn personal_replay_uses_zero_sum_deltas() {
        let tracks = by_id(vec![track(1, "A", None, None), track(2, "B", None, None)]);
        let after_one = replay_tracks(tracks.clone(), &[vote(1, 2, 1)]);
        let a_after = after_one
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(1))
            .unwrap()
            .rating;
        let b_after = after_one
            .iter()
            .find(|entry| entry.track.id == Uuid::from_u128(2))
            .unwrap()
            .rating;
        assert!((a_after + b_after - 2000.0).abs() < 1e-9);
    }
}
