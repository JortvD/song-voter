//! Small "liner note" stories derived from the active playlist: the
//! most-duelled song, the top voter, and a set of playful superlatives built
//! from the official scores and the vote history.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use axum::extract::State;
use axum::Json;
use serde::Serialize;
use serde_json::Value;
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
    pub cards: Vec<StoryCard>,
    pub leaderboards: Vec<StoryLeaderboard>,
}

#[derive(Debug, Serialize)]
pub struct StoryLeaderboard {
    pub id: &'static str,
    pub category: &'static str,
    pub title: &'static str,
    pub metric_label: &'static str,
    pub detail_label: &'static str,
    pub rows: Vec<StoryLeaderboardRow>,
}

#[derive(Debug, Serialize)]
pub struct StoryLeaderboardRow {
    pub rank: usize,
    pub title: String,
    pub subtitle: Option<String>,
    pub image_url: Option<String>,
    pub value: String,
    pub detail: String,
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

/// A generic, self-contained story card. The frontend renders these without
/// needing to know how each superlative was computed.
#[derive(Debug, Serialize)]
pub struct StoryCard {
    pub id: &'static str,
    pub category: &'static str,
    pub eyebrow: &'static str,
    pub title: String,
    pub subtitle: Option<String>,
    pub body: String,
    pub image_url: Option<String>,
    pub stats: Vec<StoryStat>,
    pub tracks: Vec<StoryTrack>,
}

#[derive(Debug, Serialize)]
pub struct StoryStat {
    pub label: &'static str,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct StoryTrack {
    pub role: &'static str,
    pub id: Uuid,
    pub title: String,
    pub artists: String,
    pub album: Option<String>,
    pub album_image_url: Option<String>,
    pub rating: f64,
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
    release_year: Option<i32>,
    decade: Option<i32>,
    album_id: Option<String>,
    album_type: Option<String>,
    total_tracks: Option<i32>,
    disc_number: Option<i32>,
    track_number: Option<i32>,
    primary_genre: Option<String>,
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
            release_year: self.release_year,
            decade: self.decade,
            album_id: self.album_id,
            album_type: self.album_type,
            total_tracks: self.total_tracks,
            disc_number: self.disc_number,
            track_number: self.track_number,
            primary_genre: self.primary_genre,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TopVoterRow {
    username: String,
    picks: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct FeaturedTrackRow {
    id: Uuid,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    rating: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct AlbumStats {
    name: String,
    artists: Option<String>,
    image_url: Option<String>,
    song_count: i64,
    avg_rating: f64,
    best_rating: f64,
    worst_rating: f64,
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
}

impl AlbumStats {
    fn spread(&self) -> f64 {
        self.best_rating - self.worst_rating
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ArtistStats {
    name: String,
    image_url: Option<String>,
    song_count: i64,
    avg_rating: f64,
    best_rating: f64,
    worst_rating: f64,
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
}

impl ArtistStats {
    fn spread(&self) -> f64 {
        self.best_rating - self.worst_rating
    }

    fn one_hit_gap(&self) -> f64 {
        self.best_rating - self.avg_rating
    }
}

#[derive(Debug, sqlx::FromRow)]
struct DecadeStats {
    decade: i32,
    image_url: Option<String>,
    song_count: i64,
    avg_rating: f64,
    best_rating: f64,
    worst_rating: f64,
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
}

impl DecadeStats {
    fn spread(&self) -> f64 {
        self.best_rating - self.worst_rating
    }
}

#[derive(Debug, sqlx::FromRow)]
struct UpsetRow {
    winner_id: Uuid,
    winner_title: String,
    winner_artists: String,
    winner_album: Option<String>,
    winner_image_url: Option<String>,
    winner_rating_before: f64,
    loser_id: Uuid,
    loser_title: String,
    loser_artists: String,
    loser_album: Option<String>,
    loser_image_url: Option<String>,
    loser_rating_before: f64,
    gap: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct CoinFlipRow {
    id: Uuid,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    rating: f64,
    comparisons: i32,
    wins: i32,
    win_rate: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct PairRow {
    track_low: Uuid,
    track_high: Uuid,
    total_votes: i64,
    low_wins: i64,
    high_wins: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct TapeStats {
    songs: i64,
    albums: i64,
    artists: i64,
    total_ms: i64,
    oldest_year: Option<i32>,
    newest_year: Option<i32>,
}

/// Public stories. Gated by The Reveal, like the standings themselves.
pub async fn public(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
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

    let key = format!("stories:public:{}", playlist.id);
    let value = state
        .cache
        .get_or_compute(key, Duration::from_secs(30), || async {
            let _permit = state.heavy_work.acquire().await.map_err(|_| {
                ApiError::service_unavailable("the server is busy; please try again")
            })?;

            let stories = load_stories(&state.db, &playlist, true).await?;
            serde_json::to_value(stories).map_err(ApiError::internal)
        })
        .await?;

    Ok(Json(value))
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

    let _permit = state
        .heavy_work
        .acquire()
        .await
        .map_err(|_| ApiError::service_unavailable("the server is busy; please try again"))?;

    Ok(Json(load_stories(&state.db, &playlist, true).await?))
}

pub async fn load_stories(
    pool: &PgPool,
    playlist: &Playlist,
    include_leaderboards: bool,
) -> Result<ChartStories, ApiError> {
    let most_duelled = sqlx::query_as::<_, StoryTrackRow>(
        "SELECT id, spotify_id, title, artists, album, album_image_url, preview_url, \
                duration_ms, release_year, decade, album_id, album_type, total_tracks, \
                disc_number, track_number, primary_genre, comparisons \
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

    let cards = load_story_cards(pool, playlist).await?;
    let leaderboards = if include_leaderboards {
        load_leaderboards(pool, playlist.id).await?
    } else {
        Vec::new()
    };

    tracing::info!(
        playlist_id = %playlist.id,
        duelled = most_duelled.as_ref().map(|story| story.track.title.as_str()).unwrap_or("-"),
        top_voter = top_voter.as_ref().map(|story| story.username.as_str()).unwrap_or("-"),
        cards = cards.len(),
        leaderboards = leaderboards.len(),
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
        cards,
        leaderboards,
    })
}

async fn load_story_cards(pool: &PgPool, playlist: &Playlist) -> Result<Vec<StoryCard>, ApiError> {
    let album_rows = load_album_stats(pool, playlist.id).await?;
    let artist_rows = load_artist_stats(pool, playlist.id).await?;
    let decade_rows = load_decade_stats(pool, playlist.id).await?;
    let upset = load_biggest_upsets(pool, playlist.id, 1)
        .await?
        .into_iter()
        .next();
    let coin_flip = load_coin_flips(pool, playlist.id, 1)
        .await?
        .into_iter()
        .next();
    let pair = load_most_argued_pairs(pool, playlist.id, 1)
        .await?
        .into_iter()
        .next();
    let tape = load_tape_stats(pool, playlist.id).await?;

    let divisive_album = most_divisive_album(&album_rows);
    let top_album = highest_rated_album(&album_rows);
    let divisive_artist = most_divisive_artist(&artist_rows);
    let top_artist = highest_rated_artist(&artist_rows);
    let one_hit_wonder = one_hit_wonder(&artist_rows);
    let best_decade = best_decade(&decade_rows);
    let divisive_decade = most_divisive_decade(&decade_rows);

    let mut track_ids = Vec::new();
    if let Some(row) = divisive_album {
        track_ids.push(row.best_id);
        track_ids.push(row.worst_id);
    }
    if let Some(row) = top_album {
        track_ids.push(row.best_id);
    }
    if let Some(row) = divisive_artist {
        track_ids.push(row.best_id);
        track_ids.push(row.worst_id);
    }
    if let Some(row) = top_artist {
        track_ids.push(row.best_id);
    }
    if let Some(row) = one_hit_wonder {
        track_ids.push(row.best_id);
    }
    if let Some(row) = best_decade {
        track_ids.push(row.best_id);
    }
    if let Some(row) = divisive_decade {
        track_ids.push(row.best_id);
        track_ids.push(row.worst_id);
    }
    if let Some(row) = &coin_flip {
        track_ids.push(row.id);
    }
    if let Some(row) = &pair {
        track_ids.push(row.track_low);
        track_ids.push(row.track_high);
    }

    let featured = fetch_featured_tracks(pool, &track_ids).await?;

    let mut cards = Vec::new();

    if let Some(row) = divisive_album {
        cards.push(StoryCard {
            id: "most-divisive-album",
            category: "album",
            eyebrow: "Most divisive album",
            title: row.name.clone(),
            subtitle: Some(format!(
                "{} · {} rated songs",
                row.artists.as_deref().unwrap_or("Unknown artist"),
                format_count(row.song_count)
            )),
            body: format!(
                "Its best and worst songs are {} points apart.",
                format_score(row.spread())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Spread", format_score(row.spread())),
                stat("Rated songs", format_count(row.song_count)),
            ],
            tracks: two_tracks(&featured, row.best_id, row.worst_id, "best", "worst"),
        });
    }

    if let Some(row) = top_album {
        cards.push(StoryCard {
            id: "top-album",
            category: "album",
            eyebrow: "Highest-rated album",
            title: row.name.clone(),
            subtitle: Some(format!(
                "{} · {} rated songs",
                row.artists.as_deref().unwrap_or("Unknown artist"),
                format_count(row.song_count)
            )),
            body: format!(
                "Averages {} across {} rated songs.",
                format_score(row.avg_rating),
                format_count(row.song_count)
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Average", format_score(row.avg_rating)),
                stat("Rated songs", format_count(row.song_count)),
                stat("Duels", format_count(row.comparisons)),
            ],
            tracks: one_track(&featured, row.best_id, "top"),
        });
    }

    if let Some(row) = divisive_artist {
        cards.push(StoryCard {
            id: "most-divisive-artist",
            category: "artist",
            eyebrow: "Most divisive artist",
            title: row.name.clone(),
            subtitle: Some(format!("{} rated songs", format_count(row.song_count))),
            body: format!(
                "A {} point gap between their best and worst song.",
                format_score(row.spread())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Spread", format_score(row.spread())),
                stat("Rated songs", format_count(row.song_count)),
            ],
            tracks: two_tracks(&featured, row.best_id, row.worst_id, "best", "worst"),
        });
    }

    if let Some(row) = top_artist {
        cards.push(StoryCard {
            id: "top-artist",
            category: "artist",
            eyebrow: "Highest-rated artist",
            title: row.name.clone(),
            subtitle: Some(format!("{} rated songs", format_count(row.song_count))),
            body: format!(
                "Averages {} across {} rated songs.",
                format_score(row.avg_rating),
                format_count(row.song_count)
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Average", format_score(row.avg_rating)),
                stat("Rated songs", format_count(row.song_count)),
                stat("Duels", format_count(row.comparisons)),
            ],
            tracks: one_track(&featured, row.best_id, "top"),
        });
    }

    if let Some(row) = one_hit_wonder {
        cards.push(StoryCard {
            id: "one-hit-wonder",
            category: "artist",
            eyebrow: "One-hit wonder",
            title: row.name.clone(),
            subtitle: Some(format!("{} rated songs", format_count(row.song_count))),
            body: format!(
                "Their best song sits {} points above their average.",
                format_score(row.one_hit_gap())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Best song", format_score(row.best_rating)),
                stat("Average", format_score(row.avg_rating)),
                stat("Gap", format_score(row.one_hit_gap())),
            ],
            tracks: one_track(&featured, row.best_id, "standout"),
        });
    }

    if let Some(row) = best_decade {
        cards.push(StoryCard {
            id: "best-decade",
            category: "decade",
            eyebrow: "Best decade",
            title: format!("The {}s", row.decade),
            subtitle: Some(format!("{} rated songs", format_count(row.song_count))),
            body: format!(
                "The {}s average {} across {} rated songs.",
                row.decade,
                format_score(row.avg_rating),
                format_count(row.song_count)
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Average", format_score(row.avg_rating)),
                stat("Rated songs", format_count(row.song_count)),
                stat("Duels", format_count(row.comparisons)),
            ],
            tracks: one_track(&featured, row.best_id, "top"),
        });
    }

    if let Some(row) = divisive_decade {
        cards.push(StoryCard {
            id: "most-divisive-decade",
            category: "decade",
            eyebrow: "Most divisive decade",
            title: format!("The {}s", row.decade),
            subtitle: Some(format!("{} rated songs", format_count(row.song_count))),
            body: format!(
                "A {} point gap between its best and worst song.",
                format_score(row.spread())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Spread", format_score(row.spread())),
                stat("Rated songs", format_count(row.song_count)),
            ],
            tracks: two_tracks(&featured, row.best_id, row.worst_id, "best", "worst"),
        });
    }

    if let Some(row) = upset {
        cards.push(StoryCard {
            id: "biggest-upset",
            category: "duel",
            eyebrow: "Biggest upset",
            title: row.winner_title.clone(),
            subtitle: Some(row.winner_artists.clone()),
            body: format!(
                "{} was {} points behind {} before the pick — and still won.",
                row.winner_title,
                format_score(row.gap),
                row.loser_title
            ),
            image_url: row.winner_image_url.clone(),
            stats: vec![
                stat("Gap", format_score(row.gap)),
                stat("Winner before", format_score(row.winner_rating_before)),
                stat("Loser before", format_score(row.loser_rating_before)),
            ],
            tracks: vec![
                StoryTrack {
                    role: "winner",
                    id: row.winner_id,
                    title: row.winner_title,
                    artists: row.winner_artists,
                    album: row.winner_album,
                    album_image_url: row.winner_image_url,
                    rating: row.winner_rating_before,
                },
                StoryTrack {
                    role: "loser",
                    id: row.loser_id,
                    title: row.loser_title,
                    artists: row.loser_artists,
                    album: row.loser_album,
                    album_image_url: row.loser_image_url,
                    rating: row.loser_rating_before,
                },
            ],
        });
    }

    if let Some(row) = coin_flip {
        cards.push(StoryCard {
            id: "coin-flip-song",
            category: "duel",
            eyebrow: "Coin-flip song",
            title: row.title.clone(),
            subtitle: Some(row.artists.clone()),
            body: format!(
                "Won {} of {} duels — almost exactly half.",
                format_count(i64::from(row.wins)),
                format_count(i64::from(row.comparisons))
            ),
            image_url: row.album_image_url.clone(),
            stats: vec![
                stat("Wins", format_count(i64::from(row.wins))),
                stat("Duels", format_count(i64::from(row.comparisons))),
                stat("Win rate", format!("{:.0}%", row.win_rate * 100.0)),
            ],
            tracks: vec![StoryTrack {
                role: "song",
                id: row.id,
                title: row.title,
                artists: row.artists,
                album: row.album,
                album_image_url: row.album_image_url,
                rating: row.rating,
            }],
        });
    }

    if let Some(row) = pair {
        let low = featured.get(&row.track_low);
        let high = featured.get(&row.track_high);
        if let (Some(low), Some(high)) = (low, high) {
            cards.push(StoryCard {
                id: "most-argued-duel",
                category: "duel",
                eyebrow: "The duel everyone argued about",
                title: format!("{} vs {}", low.title, high.title),
                subtitle: Some(format!("{} voters", format_count(row.total_votes))),
                body: format!(
                    "{} voters split this duel {}–{}; nobody could call it.",
                    format_count(row.total_votes),
                    format_count(row.low_wins),
                    format_count(row.high_wins)
                ),
                image_url: low.album_image_url.clone().or(high.album_image_url.clone()),
                stats: vec![
                    stat("Votes", format_count(row.total_votes)),
                    stat(
                        "Split",
                        format!(
                            "{}–{}",
                            format_count(row.low_wins),
                            format_count(row.high_wins)
                        ),
                    ),
                ],
                tracks: two_tracks(
                    &featured,
                    row.track_low,
                    row.track_high,
                    "one side",
                    "other side",
                ),
            });
        }
    }

    if let Some(row) = tape {
        let runtime = format_runtime(row.total_ms);
        let years = match (row.oldest_year, row.newest_year) {
            (Some(oldest), Some(newest)) => Some((oldest, newest, newest - oldest)),
            _ => None,
        };
        let body = match years {
            Some((oldest, newest, span)) => format!(
                "It runs {}, from {} to {} — a {} year span.",
                runtime,
                oldest,
                newest,
                format_count(i64::from(span))
            ),
            None => format!(
                "It runs {} across {} songs.",
                runtime,
                format_count(row.songs)
            ),
        };
        let mut stats = vec![
            stat("Songs", format_count(row.songs)),
            stat("Albums", format_count(row.albums)),
            stat("Artists", format_count(row.artists)),
            stat("Runtime", runtime.clone()),
        ];
        if let Some((_, _, span)) = years {
            stats.push(stat(
                "Year span",
                format!("{} yrs", format_count(i64::from(span))),
            ));
        }

        cards.push(StoryCard {
            id: "tape-at-a-glance",
            category: "tape",
            eyebrow: "The tape at a glance",
            title: playlist.name.clone(),
            subtitle: Some(format!(
                "{} songs · {} albums · {} artists",
                format_count(row.songs),
                format_count(row.albums),
                format_count(row.artists)
            )),
            body,
            image_url: playlist.image_url.clone(),
            stats,
            tracks: Vec::new(),
        });
    }

    Ok(cards)
}

async fn load_leaderboards(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<StoryLeaderboard>, ApiError> {
    let album_rows = load_album_stats(pool, playlist_id).await?;
    let artist_rows = load_artist_stats(pool, playlist_id).await?;
    let decade_rows = load_decade_stats(pool, playlist_id).await?;
    let upsets = load_biggest_upsets(pool, playlist_id, 20).await?;
    let coin_flips = load_coin_flips(pool, playlist_id, 5).await?;
    let pairs = load_most_argued_pairs(pool, playlist_id, 5).await?;

    let mut leaderboards = Vec::new();

    let mut divisive_albums: Vec<&AlbumStats> = album_rows
        .iter()
        .filter(|row| row.song_count >= 2)
        .collect();
    divisive_albums.sort_by(|a, b| {
        b.spread()
            .partial_cmp(&a.spread())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-albums",
        category: "album",
        title: "Most divisive albums",
        metric_label: "Spread",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_albums,
            |row| row.name.clone(),
            |row| row.artists.clone(),
            |row| row.image_url.clone(),
            |row| format_score(row.spread()),
            |row| format_count(row.song_count),
        ),
    });

    let mut top_albums: Vec<&AlbumStats> = album_rows
        .iter()
        .filter(|row| row.song_count >= 3)
        .collect();
    top_albums.sort_by(|a, b| {
        b.avg_rating
            .partial_cmp(&a.avg_rating)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| b.comparisons.cmp(&a.comparisons))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "top-albums",
        category: "album",
        title: "Highest-rated albums",
        metric_label: "Average",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &top_albums,
            |row| row.name.clone(),
            |row| row.artists.clone(),
            |row| row.image_url.clone(),
            |row| format_score(row.avg_rating),
            |row| format_count(row.song_count),
        ),
    });

    let mut divisive_artists: Vec<&ArtistStats> = artist_rows
        .iter()
        .filter(|row| row.song_count >= 3)
        .collect();
    divisive_artists.sort_by(|a, b| {
        b.spread()
            .partial_cmp(&a.spread())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-artists",
        category: "artist",
        title: "Most divisive artists",
        metric_label: "Spread",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_artists,
            |row| row.name.clone(),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_score(row.spread()),
            |row| format_count(row.song_count),
        ),
    });

    let mut top_artists: Vec<&ArtistStats> = artist_rows
        .iter()
        .filter(|row| row.song_count >= 3)
        .collect();
    top_artists.sort_by(|a, b| {
        b.avg_rating
            .partial_cmp(&a.avg_rating)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| b.comparisons.cmp(&a.comparisons))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "top-artists",
        category: "artist",
        title: "Highest-rated artists",
        metric_label: "Average",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &top_artists,
            |row| row.name.clone(),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_score(row.avg_rating),
            |row| format_count(row.song_count),
        ),
    });

    let mut one_hit_wonders: Vec<&ArtistStats> = artist_rows
        .iter()
        .filter(|row| row.song_count >= 3 && row.comparisons >= 10)
        .collect();
    one_hit_wonders.sort_by(|a, b| {
        b.one_hit_gap()
            .partial_cmp(&a.one_hit_gap())
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                b.best_rating
                    .partial_cmp(&a.best_rating)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "one-hit-wonders",
        category: "artist",
        title: "One-hit wonders",
        metric_label: "Gap",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &one_hit_wonders,
            |row| row.name.clone(),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_score(row.one_hit_gap()),
            |row| format_count(row.song_count),
        ),
    });

    let mut best_decades: Vec<&DecadeStats> = decade_rows
        .iter()
        .filter(|row| row.song_count >= 3)
        .collect();
    best_decades.sort_by(|a, b| {
        b.avg_rating
            .partial_cmp(&a.avg_rating)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.wins.cmp(&a.wins))
            .then_with(|| b.comparisons.cmp(&a.comparisons))
            .then_with(|| b.decade.cmp(&a.decade))
    });
    leaderboards.push(StoryLeaderboard {
        id: "best-decades",
        category: "decade",
        title: "Best decades",
        metric_label: "Average",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &best_decades,
            |row| format!("The {}s", row.decade),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_score(row.avg_rating),
            |row| format_count(row.song_count),
        ),
    });

    let mut divisive_decades: Vec<&DecadeStats> = decade_rows
        .iter()
        .filter(|row| row.song_count >= 5)
        .collect();
    divisive_decades.sort_by(|a, b| {
        b.spread()
            .partial_cmp(&a.spread())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| b.decade.cmp(&a.decade))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-decades",
        category: "decade",
        title: "Most divisive decades",
        metric_label: "Spread",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_decades,
            |row| format!("The {}s", row.decade),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_score(row.spread()),
            |row| format_count(row.song_count),
        ),
    });

    let mut seen_matchups = HashSet::new();
    let upset_rows: Vec<&UpsetRow> = upsets
        .iter()
        .filter(|row| seen_matchups.insert((row.winner_id, row.loser_id)))
        .collect();
    leaderboards.push(StoryLeaderboard {
        id: "biggest-upsets",
        category: "duel",
        title: "Biggest upsets",
        metric_label: "Gap",
        detail_label: "Before",
        rows: leaderboard_rows(
            &upset_rows,
            |row| row.winner_title.clone(),
            |row| Some(format!("beat {}", row.loser_title)),
            |row| row.winner_image_url.clone(),
            |row| format_score(row.gap),
            |row| {
                format!(
                    "{} vs {}",
                    format_score(row.winner_rating_before),
                    format_score(row.loser_rating_before)
                )
            },
        ),
    });

    let coin_flip_rows: Vec<&CoinFlipRow> = coin_flips.iter().collect();
    leaderboards.push(StoryLeaderboard {
        id: "coin-flip-songs",
        category: "duel",
        title: "Coin-flip songs",
        metric_label: "Win rate",
        detail_label: "Record",
        rows: leaderboard_rows(
            &coin_flip_rows,
            |row| row.title.clone(),
            |row| Some(row.artists.clone()),
            |row| row.album_image_url.clone(),
            |row| format!("{:.0}%", row.win_rate * 100.0),
            |row| {
                format!(
                    "{} of {}",
                    format_count(i64::from(row.wins)),
                    format_count(i64::from(row.comparisons))
                )
            },
        ),
    });

    let mut pair_ids = Vec::new();
    for row in &pairs {
        pair_ids.push(row.track_low);
        pair_ids.push(row.track_high);
    }
    let pair_tracks = fetch_featured_tracks(pool, &pair_ids).await?;
    let mut pair_rows = Vec::new();
    for row in &pairs {
        let (Some(low), Some(high)) = (
            pair_tracks.get(&row.track_low),
            pair_tracks.get(&row.track_high),
        ) else {
            continue;
        };
        pair_rows.push(StoryLeaderboardRow {
            rank: pair_rows.len() + 1,
            title: format!("{} vs {}", low.title, high.title),
            subtitle: None,
            image_url: low.album_image_url.clone().or(high.album_image_url.clone()),
            value: format_count(row.total_votes),
            detail: format!(
                "{}–{}",
                format_count(row.low_wins),
                format_count(row.high_wins)
            ),
        });
    }
    leaderboards.push(StoryLeaderboard {
        id: "most-argued-duels",
        category: "duel",
        title: "Most argued-about duels",
        metric_label: "Votes",
        detail_label: "Split",
        rows: pair_rows,
    });

    leaderboards.retain(|board| !board.rows.is_empty());

    Ok(leaderboards)
}

fn leaderboard_rows<T>(
    rows: &[&T],
    title: impl Fn(&T) -> String,
    subtitle: impl Fn(&T) -> Option<String>,
    image_url: impl Fn(&T) -> Option<String>,
    value: impl Fn(&T) -> String,
    detail: impl Fn(&T) -> String,
) -> Vec<StoryLeaderboardRow> {
    rows.iter()
        .take(5)
        .enumerate()
        .map(|(index, row)| StoryLeaderboardRow {
            rank: index + 1,
            title: title(*row),
            subtitle: subtitle(*row),
            image_url: image_url(*row),
            value: value(*row),
            detail: detail(*row),
        })
        .collect()
}

async fn load_album_stats(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<AlbumStats>, sqlx::Error> {
    sqlx::query_as::<_, AlbumStats>(
        "SELECT COALESCE(MAX(t.album), t.album_id) AS name, \
                MAX(t.artists) AS artists, \
                (array_agg(t.album_image_url ORDER BY t.rating DESC) \
                    FILTER (WHERE t.album_image_url IS NOT NULL))[1] AS image_url, \
                COUNT(*)::bigint AS song_count, \
                AVG(t.rating)::float8 AS avg_rating, \
                MAX(t.rating)::float8 AS best_rating, \
                MIN(t.rating)::float8 AS worst_rating, \
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins \
         FROM tracks t \
         WHERE t.playlist_id = $1 AND t.album_id IS NOT NULL AND t.comparisons > 0 \
         GROUP BY t.album_id",
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await
}

async fn load_artist_stats(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<ArtistStats>, sqlx::Error> {
    sqlx::query_as::<_, ArtistStats>(
        "SELECT a.name, \
                (array_agg(t.album_image_url ORDER BY t.rating DESC) \
                    FILTER (WHERE t.album_image_url IS NOT NULL))[1] AS image_url, \
                COUNT(*)::bigint AS song_count, \
                AVG(t.rating)::float8 AS avg_rating, \
                MAX(t.rating)::float8 AS best_rating, \
                MIN(t.rating)::float8 AS worst_rating, \
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins \
         FROM track_artists ta \
         JOIN artists a ON a.id = ta.artist_id \
         JOIN tracks t ON t.id = ta.track_id \
         WHERE t.playlist_id = $1 AND t.comparisons > 0 \
         GROUP BY a.id, a.name",
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await
}

async fn load_decade_stats(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<DecadeStats>, sqlx::Error> {
    sqlx::query_as::<_, DecadeStats>(
        "SELECT t.decade, \
                (array_agg(t.album_image_url ORDER BY t.rating DESC) \
                    FILTER (WHERE t.album_image_url IS NOT NULL))[1] AS image_url, \
                COUNT(*)::bigint AS song_count, \
                AVG(t.rating)::float8 AS avg_rating, \
                MAX(t.rating)::float8 AS best_rating, \
                MIN(t.rating)::float8 AS worst_rating, \
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins \
         FROM tracks t \
         WHERE t.playlist_id = $1 AND t.decade IS NOT NULL AND t.comparisons > 0 \
         GROUP BY t.decade",
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await
}

async fn load_biggest_upsets(
    pool: &PgPool,
    playlist_id: Uuid,
    limit: i64,
) -> Result<Vec<UpsetRow>, sqlx::Error> {
    sqlx::query_as::<_, UpsetRow>(
        "SELECT w.id AS winner_id, w.title AS winner_title, \
                w.artists AS winner_artists, w.album AS winner_album, \
                w.album_image_url AS winner_image_url, \
                CASE WHEN v.winner = v.track_low THEN v.rating_low_before \
                     ELSE v.rating_high_before END AS winner_rating_before, \
                l.id AS loser_id, l.title AS loser_title, \
                l.artists AS loser_artists, l.album AS loser_album, \
                l.album_image_url AS loser_image_url, \
                CASE WHEN v.winner = v.track_low THEN v.rating_high_before \
                     ELSE v.rating_low_before END AS loser_rating_before, \
                CASE WHEN v.winner = v.track_low \
                     THEN v.rating_high_before - v.rating_low_before \
                     ELSE v.rating_low_before - v.rating_high_before END AS gap \
         FROM votes v \
         JOIN tracks w ON w.id = v.winner \
         JOIN tracks l ON l.id = CASE WHEN v.winner = v.track_low \
                                      THEN v.track_high ELSE v.track_low END \
         WHERE v.playlist_id = $1 \
           AND (CASE WHEN v.winner = v.track_low \
                     THEN v.rating_low_before < v.rating_high_before \
                     ELSE v.rating_high_before < v.rating_low_before END) \
         ORDER BY gap DESC, v.created_at ASC \
         LIMIT $2",
    )
    .bind(playlist_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

async fn load_coin_flips(
    pool: &PgPool,
    playlist_id: Uuid,
    limit: i64,
) -> Result<Vec<CoinFlipRow>, sqlx::Error> {
    sqlx::query_as::<_, CoinFlipRow>(
        "SELECT id, title, artists, album, album_image_url, rating, comparisons, wins, \
                (wins::float8 / NULLIF(comparisons, 0)) AS win_rate \
         FROM tracks \
         WHERE playlist_id = $1 AND comparisons >= 10 \
         ORDER BY ABS((wins::float8 / NULLIF(comparisons, 0)) - 0.5) ASC, \
                  comparisons DESC, title ASC \
         LIMIT $2",
    )
    .bind(playlist_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

async fn load_most_argued_pairs(
    pool: &PgPool,
    playlist_id: Uuid,
    limit: i64,
) -> Result<Vec<PairRow>, sqlx::Error> {
    sqlx::query_as::<_, PairRow>(
        "SELECT v.track_low, v.track_high, \
                COUNT(*)::bigint AS total_votes, \
                COUNT(*) FILTER (WHERE v.winner = v.track_low)::bigint AS low_wins, \
                COUNT(*) FILTER (WHERE v.winner = v.track_high)::bigint AS high_wins \
         FROM votes v \
         WHERE v.playlist_id = $1 \
         GROUP BY v.track_low, v.track_high \
         HAVING COUNT(*) >= 3 \
         ORDER BY total_votes DESC, \
                  ABS(COUNT(*) FILTER (WHERE v.winner = v.track_low) - \
                      COUNT(*) FILTER (WHERE v.winner = v.track_high)) ASC, \
                  v.track_low ASC \
         LIMIT $2",
    )
    .bind(playlist_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

async fn load_tape_stats(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Option<TapeStats>, sqlx::Error> {
    sqlx::query_as::<_, TapeStats>(
        "SELECT COUNT(*)::bigint AS songs, \
                COUNT(DISTINCT t.album_id)::bigint AS albums, \
                (SELECT COUNT(DISTINCT ta.artist_id) \
                 FROM track_artists ta JOIN tracks t2 ON t2.id = ta.track_id \
                 WHERE t2.playlist_id = $1) AS artists, \
                COALESCE(SUM(t.duration_ms), 0)::bigint AS total_ms, \
                MIN(t.release_year) AS oldest_year, \
                MAX(t.release_year) AS newest_year \
         FROM tracks t WHERE t.playlist_id = $1",
    )
    .bind(playlist_id)
    .fetch_optional(pool)
    .await
}

async fn fetch_featured_tracks(
    pool: &PgPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, FeaturedTrackRow>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, FeaturedTrackRow>(
        "SELECT id, title, artists, album, album_image_url, rating \
         FROM tracks WHERE id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

fn most_divisive_album(rows: &[AlbumStats]) -> Option<&AlbumStats> {
    rows.iter()
        .filter(|row| row.song_count >= 2)
        .max_by(|a, b| {
            a.spread()
                .partial_cmp(&b.spread())
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.song_count.cmp(&b.song_count))
                .then_with(|| b.name.cmp(&a.name))
        })
}

fn highest_rated_album(rows: &[AlbumStats]) -> Option<&AlbumStats> {
    rows.iter()
        .filter(|row| row.song_count >= 3)
        .max_by(|a, b| {
            a.avg_rating
                .partial_cmp(&b.avg_rating)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.wins.cmp(&b.wins))
                .then_with(|| a.comparisons.cmp(&b.comparisons))
                .then_with(|| b.name.cmp(&a.name))
        })
}

fn most_divisive_artist(rows: &[ArtistStats]) -> Option<&ArtistStats> {
    rows.iter()
        .filter(|row| row.song_count >= 3)
        .max_by(|a, b| {
            a.spread()
                .partial_cmp(&b.spread())
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.song_count.cmp(&b.song_count))
                .then_with(|| b.name.cmp(&a.name))
        })
}

fn highest_rated_artist(rows: &[ArtistStats]) -> Option<&ArtistStats> {
    rows.iter()
        .filter(|row| row.song_count >= 3)
        .max_by(|a, b| {
            a.avg_rating
                .partial_cmp(&b.avg_rating)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.wins.cmp(&b.wins))
                .then_with(|| a.comparisons.cmp(&b.comparisons))
                .then_with(|| b.name.cmp(&a.name))
        })
}

fn one_hit_wonder(rows: &[ArtistStats]) -> Option<&ArtistStats> {
    rows.iter()
        .filter(|row| row.song_count >= 3 && row.comparisons >= 10)
        .max_by(|a, b| {
            a.one_hit_gap()
                .partial_cmp(&b.one_hit_gap())
                .unwrap_or(Ordering::Equal)
                .then_with(|| {
                    a.best_rating
                        .partial_cmp(&b.best_rating)
                        .unwrap_or(Ordering::Equal)
                })
                .then_with(|| b.name.cmp(&a.name))
        })
}

fn best_decade(rows: &[DecadeStats]) -> Option<&DecadeStats> {
    rows.iter()
        .filter(|row| row.song_count >= 3)
        .max_by(|a, b| {
            a.avg_rating
                .partial_cmp(&b.avg_rating)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.wins.cmp(&b.wins))
                .then_with(|| a.comparisons.cmp(&b.comparisons))
                .then_with(|| b.decade.cmp(&a.decade))
        })
}

fn most_divisive_decade(rows: &[DecadeStats]) -> Option<&DecadeStats> {
    rows.iter()
        .filter(|row| row.song_count >= 5)
        .max_by(|a, b| {
            a.spread()
                .partial_cmp(&b.spread())
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.song_count.cmp(&b.song_count))
                .then_with(|| b.decade.cmp(&a.decade))
        })
}

fn stat(label: &'static str, value: String) -> StoryStat {
    StoryStat { label, value }
}

fn one_track(
    featured: &HashMap<Uuid, FeaturedTrackRow>,
    id: Uuid,
    role: &'static str,
) -> Vec<StoryTrack> {
    featured
        .get(&id)
        .map(|row| story_track(row, role))
        .into_iter()
        .collect()
}

fn two_tracks(
    featured: &HashMap<Uuid, FeaturedTrackRow>,
    first_id: Uuid,
    second_id: Uuid,
    first_role: &'static str,
    second_role: &'static str,
) -> Vec<StoryTrack> {
    let mut tracks = Vec::new();
    if let Some(row) = featured.get(&first_id) {
        tracks.push(story_track(row, first_role));
    }
    if let Some(row) = featured.get(&second_id) {
        tracks.push(story_track(row, second_role));
    }
    tracks
}

fn story_track(row: &FeaturedTrackRow, role: &'static str) -> StoryTrack {
    StoryTrack {
        role,
        id: row.id,
        title: row.title.clone(),
        artists: row.artists.clone(),
        album: row.album.clone(),
        album_image_url: row.album_image_url.clone(),
        rating: row.rating,
    }
}

/// Formats an integer with thousands separators for the story cards.
fn format_count(value: i64) -> String {
    let negative = value < 0;
    let digits = value.abs().to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, character) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }

    if negative {
        format!("-{output}")
    } else {
        output
    }
}

fn format_score(value: f64) -> String {
    format_count(value.round() as i64)
}

fn format_runtime(total_ms: i64) -> String {
    if total_ms <= 0 {
        return "—".to_string();
    }

    let total_minutes = total_ms / 60_000;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;

    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_story_numbers() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(1_234_567), "1,234,567");
        assert_eq!(format_count(-1_200), "-1,200");
        assert_eq!(format_score(1249.6), "1,250");
    }

    #[test]
    fn formats_story_runtime() {
        assert_eq!(format_runtime(0), "—");
        assert_eq!(format_runtime(90_000), "1m");
        assert_eq!(format_runtime(3_660_000), "1h 01m");
    }
}
