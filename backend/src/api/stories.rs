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

use crate::api::admin_voters::{all_voter_stats, VoterStat};
use crate::api::settings::stats_config;
use crate::auth::AdminUser;
use crate::db::active_playlist;
use crate::error::ApiError;
use crate::models::{MatchupTrack, Playlist};
use crate::state::AppState;

/// Voter stories need a little history before a superlative is meaningful.
const VOTER_STORY_MIN_VOTES: i64 = 20;

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

#[derive(Debug, Clone, Serialize)]
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
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
    ratings: Vec<f64>,
}

impl AlbumStats {
    fn mad(&self) -> f64 {
        mean_absolute_deviation(&self.ratings)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ArtistStats {
    name: String,
    image_url: Option<String>,
    song_count: i64,
    avg_rating: f64,
    best_rating: f64,
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
    ratings: Vec<f64>,
}

impl ArtistStats {
    fn mad(&self) -> f64 {
        mean_absolute_deviation(&self.ratings)
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
    best_id: Uuid,
    worst_id: Uuid,
    comparisons: i64,
    wins: i64,
    ratings: Vec<f64>,
}

impl DecadeStats {
    fn mad(&self) -> f64 {
        mean_absolute_deviation(&self.ratings)
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

            let stories = load_stories(&state.db, &playlist, true, false).await?;
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

    Ok(Json(load_stories(&state.db, &playlist, true, true).await?))
}

pub async fn load_stories(
    pool: &PgPool,
    playlist: &Playlist,
    include_leaderboards: bool,
    include_voter_stories: bool,
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

    let cards = load_story_cards(pool, playlist, include_voter_stories).await?;
    let leaderboards = if include_leaderboards {
        load_leaderboards(pool, playlist.id, include_voter_stories).await?
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

async fn load_story_cards(
    pool: &PgPool,
    playlist: &Playlist,
    include_voter_stories: bool,
) -> Result<Vec<StoryCard>, ApiError> {
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
                "Its rated songs sit {} rating points from the album average, on average.",
                format_deviation(row.mad())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Mean deviation", format_deviation(row.mad())),
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
                "Their rated songs sit {} rating points from the artist average, on average.",
                format_deviation(row.mad())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Mean deviation", format_deviation(row.mad())),
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
                "Its rated songs sit {} rating points from the decade average, on average.",
                format_deviation(row.mad())
            ),
            image_url: row.image_url.clone(),
            stats: vec![
                stat("Mean deviation", format_deviation(row.mad())),
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

    if include_voter_stories {
        cards.extend(load_voter_story_cards(pool, playlist.id).await?);
    }

    Ok(cards)
}

/// Admin-only stories about the people doing the voting. These deliberately
/// stay out of the public Liner notes: they expose individual behaviour, not
/// just the resulting song standings.
async fn load_voter_story_cards(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<StoryCard>, ApiError> {
    let mut connection = pool.acquire().await?;
    let config = stats_config(&mut connection).await?;
    drop(connection);

    let stats = all_voter_stats(pool, playlist_id, &config).await?;
    let eligible: Vec<&VoterStat> = stats
        .iter()
        .filter(|voter| voter.votes >= VOTER_STORY_MIN_VOTES && voter.duels > 0)
        .collect();

    if eligible.is_empty() {
        return Ok(Vec::new());
    }

    let voter_ids = eligible.iter().map(|voter| voter.id).collect::<Vec<_>>();
    let signatures = load_signature_tracks(pool, playlist_id, &voter_ids).await?;
    let mut cards = Vec::new();

    if let Some(voter) = eligible.iter().copied().max_by(|a, b| {
        a.votes
            .cmp(&b.votes)
            .then_with(|| a.sessions.cmp(&b.sessions))
            .then_with(|| b.username.cmp(&a.username))
    }) {
        cards.push(voter_card(
            "voter-most-active",
            "Most active voter",
            voter,
            format!(
                "Cast {} votes across {} sessions and {} duels.",
                format_count(voter.votes),
                format_count(voter.sessions),
                format_count(voter.duels)
            ),
            vec![
                stat("Votes", format_count(voter.votes)),
                stat("Sessions", format_count(voter.sessions)),
                stat("Agreement", format_percent(voter.agreement_pct)),
                stat("Median", format_decision(voter.median_decision_seconds)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible
        .iter()
        .copied()
        .filter(|voter| voter.agreement_pct.is_some())
        .max_by(|a, b| {
            a.agreement_pct
                .partial_cmp(&b.agreement_pct)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.votes.cmp(&b.votes))
                .then_with(|| b.username.cmp(&a.username))
        })
    {
        cards.push(voter_card(
            "voter-most-agreement",
            "The crowd's North Star",
            voter,
            format!(
                "Picked the currently higher-rated song {} of the time.",
                format_percent(voter.agreement_pct)
            ),
            vec![
                stat("Agreement", format_percent(voter.agreement_pct)),
                stat("Votes", format_count(voter.votes)),
                stat("Median", format_decision(voter.median_decision_seconds)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible
        .iter()
        .copied()
        .filter(|voter| voter.agreement_pct.is_some())
        .min_by(|a, b| {
            a.agreement_pct
                .partial_cmp(&b.agreement_pct)
                .unwrap_or(Ordering::Equal)
                .then_with(|| b.votes.cmp(&a.votes))
                .then_with(|| a.username.cmp(&b.username))
        })
    {
        cards.push(voter_card(
            "voter-most-contrarian",
            "Against the grain",
            voter,
            format!(
                "Picked the currently higher-rated song only {} of the time.",
                format_percent(voter.agreement_pct)
            ),
            vec![
                stat("Agreement", format_percent(voter.agreement_pct)),
                stat("Votes", format_count(voter.votes)),
                stat("Median", format_decision(voter.median_decision_seconds)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible
        .iter()
        .copied()
        .filter(|voter| voter.median_decision_seconds.is_some())
        .min_by(|a, b| {
            a.median_decision_seconds
                .partial_cmp(&b.median_decision_seconds)
                .unwrap_or(Ordering::Equal)
                .then_with(|| b.votes.cmp(&a.votes))
                .then_with(|| a.username.cmp(&b.username))
        })
    {
        cards.push(voter_card(
            "voter-fastest",
            "Quickest thumb",
            voter,
            format!(
                "Median decision {} across {} votes.",
                format_decision(voter.median_decision_seconds),
                format_count(voter.votes)
            ),
            vec![
                stat("Median", format_decision(voter.median_decision_seconds)),
                stat("P10", format_decision(voter.p10_decision_seconds)),
                stat("Votes", format_count(voter.votes)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible
        .iter()
        .copied()
        .filter(|voter| voter.median_decision_seconds.is_some())
        .max_by(|a, b| {
            a.median_decision_seconds
                .partial_cmp(&b.median_decision_seconds)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.votes.cmp(&b.votes))
                .then_with(|| b.username.cmp(&a.username))
        })
    {
        cards.push(voter_card(
            "voter-slowest",
            "Deep listener",
            voter,
            format!(
                "Takes a median of {} to decide, across {} votes.",
                format_decision(voter.median_decision_seconds),
                format_count(voter.votes)
            ),
            vec![
                stat("Median", format_decision(voter.median_decision_seconds)),
                stat("P90", format_decision(voter.p90_decision_seconds)),
                stat("Votes", format_count(voter.votes)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible.iter().copied().max_by(|a, b| {
        a.avg_duels_per_session
            .total_cmp(&b.avg_duels_per_session)
            .then_with(|| a.sessions.cmp(&b.sessions))
            .then_with(|| b.username.cmp(&a.username))
    }) {
        cards.push(voter_card(
            "voter-longest-session",
            "Marathon session",
            voter,
            format!(
                "Averages {:.1} duels per session across {} sessions.",
                voter.avg_duels_per_session,
                format_count(voter.sessions)
            ),
            vec![
                stat(
                    "Duels / session",
                    format!("{:.1}", voter.avg_duels_per_session),
                ),
                stat("Sessions", format_count(voter.sessions)),
                stat(
                    "Total time",
                    format_duration_seconds(voter.total_voting_time_seconds),
                ),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible.iter().copied().max_by(|a, b| {
        a.distinct_songs_seen
            .cmp(&b.distinct_songs_seen)
            .then_with(|| a.votes.cmp(&b.votes))
            .then_with(|| b.username.cmp(&a.username))
    }) {
        cards.push(voter_card(
            "voter-widest-taste",
            "Widest taste",
            voter,
            format!(
                "Has seen {} distinct songs across {} duels.",
                format_count(voter.distinct_songs_seen),
                format_count(voter.duels)
            ),
            vec![
                stat("Songs seen", format_count(voter.distinct_songs_seen)),
                stat("Votes", format_count(voter.votes)),
                stat("Sessions", format_count(voter.sessions)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible
        .iter()
        .copied()
        .filter(|voter| voter.side_bias_pct.is_some())
        .max_by(|a, b| {
            a.side_bias_pct
                .partial_cmp(&b.side_bias_pct)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.votes.cmp(&b.votes))
                .then_with(|| b.username.cmp(&a.username))
        })
    {
        cards.push(voter_card(
            "voter-left-lean",
            "Leaning left",
            voter,
            format!(
                "Made {} of their picks on Side A.",
                format_percent(voter.side_bias_pct)
            ),
            vec![
                stat("Side A", format_percent(voter.side_bias_pct)),
                stat("Votes", format_count(voter.votes)),
                stat("Agreement", format_percent(voter.agreement_pct)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    if let Some(voter) = eligible.iter().copied().max_by(|a, b| {
        a.total_voting_time_seconds
            .total_cmp(&b.total_voting_time_seconds)
            .then_with(|| a.sessions.cmp(&b.sessions))
            .then_with(|| b.username.cmp(&a.username))
    }) {
        cards.push(voter_card(
            "voter-most-time",
            "Most time in the room",
            voter,
            format!(
                "Spent {} in voting sessions across {} sessions.",
                format_duration_seconds(voter.total_voting_time_seconds),
                format_count(voter.sessions)
            ),
            vec![
                stat(
                    "Voting time",
                    format_duration_seconds(voter.total_voting_time_seconds),
                ),
                stat("Sessions", format_count(voter.sessions)),
                stat("Votes", format_count(voter.votes)),
            ],
            signature_for(&signatures, voter),
        ));
    }

    let decisions = eligible
        .iter()
        .filter_map(|voter| voter.median_decision_seconds)
        .collect::<Vec<_>>();
    let agreements = eligible
        .iter()
        .filter_map(|voter| voter.agreement_pct)
        .collect::<Vec<_>>();
    let sessions = eligible
        .iter()
        .map(|voter| voter.sessions as f64)
        .collect::<Vec<_>>();
    let per_session = eligible
        .iter()
        .map(|voter| voter.avg_duels_per_session)
        .collect::<Vec<_>>();

    if let (Some(median_decision), Some(median_agreement)) = (
        crate::api::analytics::median(&decisions),
        crate::api::analytics::median(&agreements),
    ) {
        cards.push(StoryCard {
            id: "voter-crowd",
            category: "voter",
            eyebrow: "The room's habits",
            title: "How the room votes".to_string(),
            subtitle: Some(format!(
                "{} voters with at least {} votes",
                format_count(eligible.len() as i64),
                format_count(VOTER_STORY_MIN_VOTES)
            )),
            body: format!(
                "The median voter decides in {} and agrees with the current standings {} of the time.",
                format_decision(Some(median_decision)),
                format_percent(Some(median_agreement))
            ),
            image_url: None,
            stats: vec![
                stat("Median decision", format_decision(Some(median_decision))),
                stat("Median agreement", format_percent(Some(median_agreement))),
                stat(
                    "Median sessions",
                    format_count(crate::api::analytics::median(&sessions).unwrap_or(0.0).round() as i64),
                ),
                stat(
                    "Duels / session",
                    format!(
                        "{:.1}",
                        crate::api::analytics::median(&per_session).unwrap_or(0.0)
                    ),
                ),
            ],
            tracks: Vec::new(),
        });
    }

    Ok(cards)
}

fn voter_card(
    id: &'static str,
    eyebrow: &'static str,
    voter: &VoterStat,
    body: String,
    stats: Vec<StoryStat>,
    tracks: Vec<StoryTrack>,
) -> StoryCard {
    StoryCard {
        id,
        category: "voter",
        eyebrow,
        title: voter.username.clone(),
        subtitle: Some(format!(
            "{} votes · {} sessions",
            format_count(voter.votes),
            format_count(voter.sessions)
        )),
        body,
        image_url: None,
        stats,
        tracks,
    }
}

fn signature_for(signatures: &HashMap<Uuid, StoryTrack>, voter: &VoterStat) -> Vec<StoryTrack> {
    signatures
        .get(&voter.id)
        .cloned()
        .map(|track| vec![track])
        .unwrap_or_default()
}

#[derive(Debug, sqlx::FromRow)]
struct SignatureRow {
    user_id: Uuid,
    id: Uuid,
    title: String,
    artists: String,
    album: Option<String>,
    album_image_url: Option<String>,
    rating: f64,
}

async fn load_signature_tracks(
    pool: &PgPool,
    playlist_id: Uuid,
    user_ids: &[Uuid],
) -> Result<HashMap<Uuid, StoryTrack>, ApiError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, SignatureRow>(
        "WITH picks AS ( \
             SELECT v.user_id, t.id, t.title, t.artists, t.album, t.album_image_url, t.rating, \
                    ROW_NUMBER() OVER ( \
                        PARTITION BY v.user_id \
                        ORDER BY COUNT(*) DESC, MAX(v.created_at) DESC \
                    ) AS rank \
             FROM votes v \
             JOIN tracks t ON t.id = v.winner \
             WHERE v.playlist_id = $1 AND v.user_id = ANY($2) \
             GROUP BY v.user_id, t.id, t.title, t.artists, t.album, t.album_image_url, t.rating \
         ) \
         SELECT user_id, id, title, artists, album, album_image_url, rating \
         FROM picks WHERE rank = 1",
    )
    .bind(playlist_id)
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            (
                row.user_id,
                StoryTrack {
                    role: "signature pick",
                    id: row.id,
                    title: row.title,
                    artists: row.artists,
                    album: row.album,
                    album_image_url: row.album_image_url,
                    rating: row.rating,
                },
            )
        })
        .collect())
}

async fn load_voter_leaderboards(
    pool: &PgPool,
    playlist_id: Uuid,
) -> Result<Vec<StoryLeaderboard>, ApiError> {
    let mut connection = pool.acquire().await?;
    let config = stats_config(&mut connection).await?;
    drop(connection);

    let stats = all_voter_stats(pool, playlist_id, &config).await?;
    let eligible: Vec<&VoterStat> = stats
        .iter()
        .filter(|voter| voter.votes >= VOTER_STORY_MIN_VOTES && voter.duels > 0)
        .collect();
    if eligible.is_empty() {
        return Ok(Vec::new());
    }

    let mut boards = Vec::new();

    let mut active = eligible.clone();
    active.sort_by(|a, b| {
        b.votes
            .cmp(&a.votes)
            .then_with(|| b.sessions.cmp(&a.sessions))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "most-active-voters",
        category: "voter",
        title: "Most active voters",
        metric_label: "Votes",
        detail_label: "Sessions",
        rows: leaderboard_rows(
            &active,
            |row| row.username.clone(),
            |row| Some(format!("{} duels", format_count(row.duels))),
            |_| None,
            |row| format_count(row.votes),
            |row| format_count(row.sessions),
        ),
    });

    let mut agreement = eligible
        .iter()
        .copied()
        .filter(|voter| voter.agreement_pct.is_some())
        .collect::<Vec<_>>();
    agreement.sort_by(|a, b| {
        b.agreement_pct
            .partial_cmp(&a.agreement_pct)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.votes.cmp(&a.votes))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "voter-agreement",
        category: "voter",
        title: "Most agreement with the crowd",
        metric_label: "Agreement",
        detail_label: "Votes",
        rows: leaderboard_rows(
            &agreement,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format_percent(row.agreement_pct),
            |row| format_count(row.votes),
        ),
    });

    let mut quickest = eligible
        .iter()
        .copied()
        .filter(|voter| voter.median_decision_seconds.is_some())
        .collect::<Vec<_>>();
    quickest.sort_by(|a, b| {
        a.median_decision_seconds
            .partial_cmp(&b.median_decision_seconds)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.votes.cmp(&a.votes))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "quickest-voters",
        category: "voter",
        title: "Quickest decisions",
        metric_label: "Median",
        detail_label: "Votes",
        rows: leaderboard_rows(
            &quickest,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format_decision(row.median_decision_seconds),
            |row| format_count(row.votes),
        ),
    });

    let mut longest = eligible.clone();
    longest.sort_by(|a, b| {
        b.avg_duels_per_session
            .total_cmp(&a.avg_duels_per_session)
            .then_with(|| b.sessions.cmp(&a.sessions))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "longest-voter-sessions",
        category: "voter",
        title: "Longest average sessions",
        metric_label: "Duels / session",
        detail_label: "Sessions",
        rows: leaderboard_rows(
            &longest,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format!("{:.1}", row.avg_duels_per_session),
            |row| format_count(row.sessions),
        ),
    });

    let mut taste = eligible.clone();
    taste.sort_by(|a, b| {
        b.distinct_songs_seen
            .cmp(&a.distinct_songs_seen)
            .then_with(|| b.votes.cmp(&a.votes))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "widest-voter-taste",
        category: "voter",
        title: "Widest taste",
        metric_label: "Songs seen",
        detail_label: "Votes",
        rows: leaderboard_rows(
            &taste,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format_count(row.distinct_songs_seen),
            |row| format_count(row.votes),
        ),
    });

    let mut timing = eligible.clone();
    timing.sort_by(|a, b| {
        b.total_voting_time_seconds
            .total_cmp(&a.total_voting_time_seconds)
            .then_with(|| b.sessions.cmp(&a.sessions))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "most-voter-time",
        category: "voter",
        title: "Most time in the room",
        metric_label: "Voting time",
        detail_label: "Sessions",
        rows: leaderboard_rows(
            &timing,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format_duration_seconds(row.total_voting_time_seconds),
            |row| format_count(row.sessions),
        ),
    });

    let mut lean = eligible
        .iter()
        .copied()
        .filter(|voter| voter.side_bias_pct.is_some())
        .collect::<Vec<_>>();
    lean.sort_by(|a, b| {
        b.side_bias_pct
            .partial_cmp(&a.side_bias_pct)
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.votes.cmp(&a.votes))
            .then_with(|| a.username.cmp(&b.username))
    });
    boards.push(StoryLeaderboard {
        id: "voter-side-lean",
        category: "voter",
        title: "Most left-leaning voters",
        metric_label: "Side A",
        detail_label: "Votes",
        rows: leaderboard_rows(
            &lean,
            |row| row.username.clone(),
            |_| None,
            |_| None,
            |row| format_percent(row.side_bias_pct),
            |row| format_count(row.votes),
        ),
    });

    Ok(boards)
}

async fn load_leaderboards(
    pool: &PgPool,
    playlist_id: Uuid,
    include_voter_stories: bool,
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
        b.mad()
            .partial_cmp(&a.mad())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-albums",
        category: "album",
        title: "Most divisive albums",
        metric_label: "Mean deviation",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_albums,
            |row| row.name.clone(),
            |row| row.artists.clone(),
            |row| row.image_url.clone(),
            |row| format_deviation(row.mad()),
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
        b.mad()
            .partial_cmp(&a.mad())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| a.name.cmp(&b.name))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-artists",
        category: "artist",
        title: "Most divisive artists",
        metric_label: "Mean deviation",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_artists,
            |row| row.name.clone(),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_deviation(row.mad()),
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
        b.mad()
            .partial_cmp(&a.mad())
            .unwrap_or(Ordering::Equal)
            .then_with(|| b.song_count.cmp(&a.song_count))
            .then_with(|| b.decade.cmp(&a.decade))
    });
    leaderboards.push(StoryLeaderboard {
        id: "divisive-decades",
        category: "decade",
        title: "Most divisive decades",
        metric_label: "Mean deviation",
        detail_label: "Songs",
        rows: leaderboard_rows(
            &divisive_decades,
            |row| format!("The {}s", row.decade),
            |_| None,
            |row| row.image_url.clone(),
            |row| format_deviation(row.mad()),
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

    if include_voter_stories {
        leaderboards.extend(load_voter_leaderboards(pool, playlist_id).await?);
    }

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
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins, \
                array_agg(t.rating) AS ratings \
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
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins, \
                array_agg(t.rating) AS ratings \
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
                (array_agg(t.id ORDER BY t.rating DESC))[1] AS best_id, \
                (array_agg(t.id ORDER BY t.rating ASC))[1] AS worst_id, \
                COALESCE(SUM(t.comparisons), 0)::bigint AS comparisons, \
                COALESCE(SUM(t.wins), 0)::bigint AS wins, \
                array_agg(t.rating) AS ratings \
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
            a.mad()
                .partial_cmp(&b.mad())
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
            a.mad()
                .partial_cmp(&b.mad())
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
            a.mad()
                .partial_cmp(&b.mad())
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
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
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

fn format_deviation(value: f64) -> String {
    format!("{value:.1}")
}

fn mean_absolute_deviation(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    values.iter().map(|value| (value - mean).abs()).sum::<f64>() / values.len() as f64
}

fn format_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "—".to_string())
}

fn format_decision(value: Option<f64>) -> String {
    match value {
        Some(value) if value < 10.0 => format!("{value:.1}s"),
        Some(value) if value < 60.0 => format!("{value:.0}s"),
        Some(value) => format!(
            "{}m {}s",
            (value / 60.0).floor() as i64,
            (value % 60.0).round() as i64
        ),
        None => "—".to_string(),
    }
}

fn format_duration_seconds(value: f64) -> String {
    if !value.is_finite() || value <= 0.0 {
        return "—".to_string();
    }
    let total_seconds = value.round() as i64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
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

    #[test]
    fn divisive_metric_is_mean_absolute_deviation() {
        assert!((mean_absolute_deviation(&[1000.0, 1000.0, 1000.0]) - 0.0).abs() < 1e-9);
        assert!((mean_absolute_deviation(&[900.0, 1000.0, 1100.0]) - 66.6666666667).abs() < 1e-6);
        assert!((mean_absolute_deviation(&[1000.0, 1200.0]) - 100.0).abs() < 1e-9);
        assert_eq!(mean_absolute_deviation(&[]), 0.0);
        assert_eq!(format_deviation(66.666), "66.7");
    }

    #[test]
    fn formats_voter_story_values() {
        assert_eq!(format_percent(Some(61.26)), "61.3%");
        assert_eq!(format_percent(None), "—");
        assert_eq!(format_decision(Some(1.24)), "1.2s");
        assert_eq!(format_decision(Some(34.6)), "35s");
        assert_eq!(format_decision(Some(75.0)), "1m 15s");
        assert_eq!(format_decision(None), "—");
        assert_eq!(format_duration_seconds(0.0), "—");
        assert_eq!(format_duration_seconds(90.0), "1m");
        assert_eq!(format_duration_seconds(3_660.0), "1h 1m");
    }
}
