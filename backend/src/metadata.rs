//! Music metadata helpers: release years, decades and a small, stable broad
//! genre mapping on top of Spotify's free-form artist genres.

use std::collections::HashMap;

use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::spotify::{ImportedArtist, ImportedTrack};

pub const BROAD_GENRES: &[&str] = &[
    "Pop",
    "Rock",
    "Hip-Hop/Rap",
    "Electronic/Dance",
    "R&B/Soul",
    "Indie/Alternative",
    "Folk/Country",
    "Jazz",
    "Classical",
    "Metal/Punk",
    "Latin",
    "World",
    "Soundtrack",
    "Other",
];

/// Map one Spotify artist genre to the broad genre used by filters and stats.
pub fn broad_genre(genre: &str) -> &'static str {
    let value = genre.trim().to_ascii_lowercase();

    // Specific families first so "indie folk" still reads as folk, while
    // "indie rock" reads as indie/alternative.
    let rules: &[(&[&str], &str)] = &[
        (
            &["soundtrack", "score", "film", "musical", "cinematic"],
            "Soundtrack",
        ),
        (
            &["classical", "orchestra", "opera", "chamber", "baroque"],
            "Classical",
        ),
        (
            &["metal", "hardcore", "punk", "grunge", "emo"],
            "Metal/Punk",
        ),
        (
            &["hip hop", "hip-hop", "rap", "trap", "drill", "grime"],
            "Hip-Hop/Rap",
        ),
        (
            &["r&b", "rnb", "soul", "funk", "motown", "neo soul", "disco"],
            "R&B/Soul",
        ),
        (
            &[
                "electronic",
                "edm",
                "house",
                "techno",
                "trance",
                "dubstep",
                "drum and bass",
                "dnb",
                "garage",
                "synth",
                "ambient",
                "idm",
            ],
            "Electronic/Dance",
        ),
        (&["jazz", "bebop", "swing", "blues"], "Jazz"),
        (
            &[
                "latin",
                "reggaeton",
                "salsa",
                "cumbia",
                "bachata",
                "merengue",
            ],
            "Latin",
        ),
        (
            &[
                "reggae",
                "ska",
                "dancehall",
                "afro",
                "world",
                "global",
                "celtic",
                "amapiano",
            ],
            "World",
        ),
        (
            &[
                "folk",
                "country",
                "americana",
                "bluegrass",
                "singer-songwriter",
                "acoustic",
            ],
            "Folk/Country",
        ),
        (
            &[
                "indie",
                "alternative",
                "alt ",
                "shoegaze",
                "dream pop",
                "lo-fi",
            ],
            "Indie/Alternative",
        ),
        (&["pop"], "Pop"),
        (&["rock"], "Rock"),
    ];

    for (keywords, broad) in rules {
        if keywords.iter().any(|keyword| value.contains(keyword)) {
            return broad;
        }
    }

    "Other"
}

/// Pick the dominant broad genre from an artist's raw Spotify genres.
pub fn primary_genre(genres: &[String]) -> Option<&'static str> {
    if genres.is_empty() {
        return None;
    }

    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    let mut order: Vec<&'static str> = Vec::new();

    for genre in genres {
        let broad = broad_genre(genre);
        if !counts.contains_key(broad) {
            order.push(broad);
        }
        *counts.entry(broad).or_default() += 1;
    }

    order
        .into_iter()
        .max_by_key(|genre| counts.get(genre).copied().unwrap_or(0))
}

pub fn decade_from_year(year: i32) -> i32 {
    (year / 10) * 10
}

/// Spotify can return "1981", "1981-12" or "1981-12-31".
pub fn parse_release_year(value: &str) -> Option<i32> {
    let year = value.get(0..4)?.parse::<i32>().ok()?;
    if (1000..=3000).contains(&year) {
        Some(year)
    } else {
        None
    }
}

/// Upsert the artists referenced by imported tracks and rebuild the
/// track-to-artist links for this playlist.
pub async fn sync_track_artists(
    pool: &PgPool,
    playlist_id: Uuid,
    tracks: &[ImportedTrack],
) -> Result<(), sqlx::Error> {
    let mut unique: HashMap<String, String> = HashMap::new();
    for track in tracks {
        for artist in &track.artist_refs {
            if let Some(spotify_id) = &artist.spotify_id {
                unique
                    .entry(spotify_id.clone())
                    .or_insert_with(|| artist.name.clone());
            }
        }
    }

    if unique.is_empty() {
        return Ok(());
    }

    let artists: Vec<(String, String)> = unique.into_iter().collect();
    for chunk in artists.chunks(500) {
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO artists (spotify_id, name) ");
        builder.push_values(chunk, |mut row, (spotify_id, name)| {
            row.push_bind(spotify_id.as_str()).push_bind(name.as_str());
        });
        builder.push(
            " ON CONFLICT (spotify_id) DO UPDATE SET \
                name = EXCLUDED.name, updated_at = now()",
        );
        builder.build().execute(pool).await?;
    }

    let spotify_ids: Vec<String> = artists.iter().map(|(id, _)| id.clone()).collect();
    let artist_rows = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, spotify_id FROM artists WHERE spotify_id = ANY($1)",
    )
    .bind(&spotify_ids)
    .fetch_all(pool)
    .await?;
    let artist_by_spotify: HashMap<String, Uuid> = artist_rows
        .into_iter()
        .map(|(id, spotify_id)| (spotify_id, id))
        .collect();

    let track_rows = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, spotify_id FROM tracks WHERE playlist_id = $1",
    )
    .bind(playlist_id)
    .fetch_all(pool)
    .await?;
    let track_by_spotify: HashMap<String, Uuid> = track_rows
        .into_iter()
        .map(|(id, spotify_id)| (spotify_id, id))
        .collect();

    let imported_spotify_ids: Vec<String> = tracks
        .iter()
        .map(|track| track.spotify_id.clone())
        .collect();

    // Only rebuild links for tracks present in this import. Tracks removed from
    // Spotify are intentionally retained for history, and their artist/genre
    // links must survive a re-sync too.
    sqlx::query(
        "DELETE FROM track_artists USING tracks \
         WHERE track_artists.track_id = tracks.id \
           AND tracks.playlist_id = $1 \
           AND tracks.spotify_id = ANY($2)",
    )
    .bind(playlist_id)
    .bind(&imported_spotify_ids)
    .execute(pool)
    .await?;

    let mut relations: Vec<(Uuid, Uuid, i32)> = Vec::new();
    for track in tracks {
        let Some(track_id) = track_by_spotify.get(&track.spotify_id) else {
            continue;
        };

        for (position, artist) in track.artist_refs.iter().enumerate() {
            let Some(spotify_id) = &artist.spotify_id else {
                continue;
            };
            let Some(artist_id) = artist_by_spotify.get(spotify_id) else {
                continue;
            };
            relations.push((*track_id, *artist_id, position as i32));
        }
    }

    for chunk in relations.chunks(1000) {
        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("INSERT INTO track_artists (track_id, artist_id, position) ");
        builder.push_values(chunk, |mut row, (track_id, artist_id, position)| {
            row.push_bind(*track_id)
                .push_bind(*artist_id)
                .push_bind(*position);
        });
        builder
            .push(" ON CONFLICT (track_id, artist_id) DO UPDATE SET position = EXCLUDED.position");
        builder.build().execute(pool).await?;
    }

    tracing::info!(
        %playlist_id,
        artists = artist_by_spotify.len(),
        links = relations.len(),
        "track artists synced"
    );

    Ok(())
}

/// Update the metadata columns of existing tracks from a fresh playlist fetch.
pub async fn apply_track_metadata(
    pool: &PgPool,
    playlist_id: Uuid,
    tracks: &[ImportedTrack],
) -> Result<(), sqlx::Error> {
    for track in tracks {
        let decade = track.release_year.map(decade_from_year);

        sqlx::query(
            "UPDATE tracks SET \
                title = $1, artists = $2, album = $3, \
                album_image_url = COALESCE($4, album_image_url), \
                preview_url = COALESCE(preview_url, $5), \
                duration_ms = COALESCE(duration_ms, $6), \
                release_date = $7, release_date_precision = $8, \
                release_year = $9, decade = $10, album_id = $11, \
                album_type = $12, total_tracks = $13, \
                disc_number = $14, track_number = $15, \
                position = $16 \
             WHERE playlist_id = $17 AND spotify_id = $18",
        )
        .bind(&track.title)
        .bind(&track.artists)
        .bind(track.album.as_deref())
        .bind(track.album_image_url.as_deref())
        .bind(track.preview_url.as_deref())
        .bind(track.duration_ms)
        .bind(track.release_date.as_deref())
        .bind(track.release_date_precision.as_deref())
        .bind(track.release_year)
        .bind(decade)
        .bind(track.album_id.as_deref())
        .bind(track.album_type.as_deref())
        .bind(track.total_tracks)
        .bind(track.disc_number)
        .bind(track.track_number)
        .bind(track.position)
        .bind(playlist_id)
        .bind(&track.spotify_id)
        .execute(pool)
        .await?;
    }

    tracing::info!(
        %playlist_id,
        tracks = tracks.len(),
        "track metadata applied"
    );

    Ok(())
}

pub async fn update_artist_genres(
    pool: &PgPool,
    artists: &[ImportedArtist],
) -> Result<(), sqlx::Error> {
    for artist in artists {
        let broad = primary_genre(&artist.genres);

        sqlx::query(
            "UPDATE artists SET name = $1, genres = $2, primary_genre = $3, updated_at = now() \
             WHERE spotify_id = $4",
        )
        .bind(&artist.name)
        .bind(&artist.genres)
        .bind(broad)
        .bind(&artist.spotify_id)
        .execute(pool)
        .await?;
    }

    tracing::info!(artists = artists.len(), "artist genres applied");
    Ok(())
}

/// Recompute each track's broad genre from its linked artists. The most common
/// artist genre wins; songs with no genre data stay NULL.
pub async fn refresh_track_genres(pool: &PgPool, playlist_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE tracks SET primary_genre = NULL WHERE playlist_id = $1")
        .bind(playlist_id)
        .execute(pool)
        .await?;

    sqlx::query(
        "UPDATE tracks t SET primary_genre = sub.primary_genre \
         FROM ( \
             SELECT ta.track_id, mode() WITHIN GROUP (ORDER BY a.primary_genre) AS primary_genre \
             FROM track_artists ta \
             JOIN artists a ON a.id = ta.artist_id \
             JOIN tracks t2 ON t2.id = ta.track_id \
             WHERE t2.playlist_id = $1 AND a.primary_genre IS NOT NULL \
             GROUP BY ta.track_id \
         ) sub \
         WHERE t.id = sub.track_id",
    )
    .bind(playlist_id)
    .execute(pool)
    .await?;

    tracing::info!(%playlist_id, "track genres refreshed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_genres_to_stable_buckets() {
        assert_eq!(broad_genre("indie rock"), "Indie/Alternative");
        assert_eq!(broad_genre("indie folk"), "Folk/Country");
        assert_eq!(broad_genre("hip hop"), "Hip-Hop/Rap");
        assert_eq!(broad_genre("deep house"), "Electronic/Dance");
        assert_eq!(broad_genre("neo soul"), "R&B/Soul");
        assert_eq!(broad_genre("progressive metal"), "Metal/Punk");
        assert_eq!(broad_genre("film score"), "Soundtrack");
        assert_eq!(broad_genre("unknown thing"), "Other");
    }

    #[test]
    fn picks_the_dominant_artist_genre() {
        let genres = vec![
            "pop".to_string(),
            "dance pop".to_string(),
            "rock".to_string(),
        ];
        assert_eq!(primary_genre(&genres), Some("Pop"));
        assert_eq!(primary_genre(&[]), None);
    }

    #[test]
    fn parses_release_years_and_decades() {
        assert_eq!(parse_release_year("1981"), Some(1981));
        assert_eq!(parse_release_year("1981-12"), Some(1981));
        assert_eq!(parse_release_year("1981-12-31"), Some(1981));
        assert_eq!(parse_release_year("not-a-date"), None);
        assert_eq!(decade_from_year(1981), 1980);
        assert_eq!(decade_from_year(2000), 2000);
    }

    macro_rules! genre_case {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                assert_eq!(broad_genre($input), $expected);
            }
        };
    }

    genre_case!(maps_soundtrack_keywords, "film score", "Soundtrack");
    genre_case!(maps_musical_theatre, "musical theatre", "Soundtrack");
    genre_case!(maps_cinematic_music, "cinematic", "Soundtrack");
    genre_case!(maps_classical_keywords, "classical", "Classical");
    genre_case!(maps_orchestral_music, "orchestral", "Classical");
    genre_case!(maps_opera, "opera", "Classical");
    genre_case!(maps_chamber_music, "chamber ensemble", "Classical");
    genre_case!(maps_baroque, "baroque", "Classical");
    genre_case!(maps_metal, "metal", "Metal/Punk");
    genre_case!(maps_hardcore_punk, "hardcore punk", "Metal/Punk");
    genre_case!(maps_punk_rock, "punk rock", "Metal/Punk");
    genre_case!(maps_grunge, "grunge", "Metal/Punk");
    genre_case!(maps_emo, "emo", "Metal/Punk");
    genre_case!(maps_hip_hop, "hip hop", "Hip-Hop/Rap");
    genre_case!(maps_rap, "rap", "Hip-Hop/Rap");
    genre_case!(maps_trap, "trap", "Hip-Hop/Rap");
    genre_case!(maps_drill, "drill", "Hip-Hop/Rap");
    genre_case!(maps_grime, "grime", "Hip-Hop/Rap");
    genre_case!(maps_rnb, "r&b", "R&B/Soul");
    genre_case!(maps_soul, "soul", "R&B/Soul");
    genre_case!(maps_funk, "funk", "R&B/Soul");
    genre_case!(maps_motown, "motown", "R&B/Soul");
    genre_case!(maps_neo_soul, "neo soul", "R&B/Soul");
    genre_case!(maps_disco, "disco", "R&B/Soul");
    genre_case!(maps_edm, "edm", "Electronic/Dance");
    genre_case!(maps_house, "deep house", "Electronic/Dance");
    genre_case!(maps_techno, "techno", "Electronic/Dance");
    genre_case!(maps_trance, "trance", "Electronic/Dance");
    genre_case!(maps_dubstep, "dubstep", "Electronic/Dance");
    genre_case!(maps_dnb, "dnb", "Electronic/Dance");
    genre_case!(maps_ambient, "ambient", "Electronic/Dance");
    genre_case!(maps_idm, "idm", "Electronic/Dance");
    genre_case!(maps_jazz, "jazz", "Jazz");
    genre_case!(maps_bebop, "bebop", "Jazz");
    genre_case!(maps_swing, "swing", "Jazz");
    genre_case!(maps_blues, "blues", "Jazz");
    genre_case!(maps_latin, "latin", "Latin");
    genre_case!(maps_reggaeton, "reggaeton", "Latin");
    genre_case!(maps_salsa, "salsa", "Latin");
    genre_case!(maps_cumbia, "cumbia", "Latin");
    genre_case!(maps_bachata, "bachata", "Latin");
    genre_case!(maps_merengue, "merengue", "Latin");
    genre_case!(maps_reggae, "reggae", "World");
    genre_case!(maps_afrobeat, "afrobeat", "World");
    genre_case!(maps_celtic, "celtic", "World");
    genre_case!(maps_amapiano, "amapiano", "World");
    genre_case!(maps_folk, "folk", "Folk/Country");
    genre_case!(maps_country, "country", "Folk/Country");
    genre_case!(maps_americana, "americana", "Folk/Country");
    genre_case!(maps_bluegrass, "bluegrass", "Folk/Country");
    genre_case!(maps_acoustic, "acoustic", "Folk/Country");
    genre_case!(maps_indie_rock, "indie rock", "Indie/Alternative");
    genre_case!(maps_alternative, "alternative", "Indie/Alternative");
    genre_case!(maps_shoegaze, "shoegaze", "Indie/Alternative");
    genre_case!(maps_dream_pop, "dream pop", "Indie/Alternative");
    genre_case!(maps_lo_fi, "lo-fi beats", "Indie/Alternative");
    genre_case!(maps_pop, "pop", "Pop");
    genre_case!(maps_k_pop, "k-pop", "Pop");
    genre_case!(maps_rock, "rock", "Rock");
    genre_case!(maps_classic_rock, "classic rock", "Rock");
    genre_case!(maps_unknown_to_other, "field recordings", "Other");
    genre_case!(maps_empty_to_other, "", "Other");

    #[test]
    fn genre_matching_is_case_insensitive_and_trims() {
        assert_eq!(broad_genre("  INDIE ROCK  "), "Indie/Alternative");
        assert_eq!(broad_genre("Hip-Hop"), "Hip-Hop/Rap");
    }

    #[test]
    fn specific_families_win_over_generic_words() {
        // "indie folk" must stay folk even though it contains "indie".
        assert_eq!(broad_genre("indie folk"), "Folk/Country");
        // "folk rock" must stay folk even though it contains "rock".
        assert_eq!(broad_genre("folk rock"), "Folk/Country");
        // "pop punk" is punk, not pop.
        assert_eq!(broad_genre("pop punk"), "Metal/Punk");
    }

    #[test]
    fn every_broad_genre_bucket_is_unique_and_reachable() {
        let mut seen = std::collections::HashSet::new();
        for genre in BROAD_GENRES {
            assert!(seen.insert(*genre), "duplicate bucket {genre}");
        }
        assert_eq!(seen.len(), BROAD_GENRES.len());
        for genre in BROAD_GENRES {
            let input = genre.to_ascii_lowercase();
            if input == "other" {
                assert_eq!(broad_genre("unknown"), "Other");
            } else {
                assert_eq!(
                    broad_genre(&input),
                    *genre,
                    "bucket {genre} must map to itself"
                );
            }
        }
    }

    #[test]
    fn primary_genre_picks_the_most_frequent_bucket() {
        let genres = vec![
            "alternative rock".to_string(),
            "indie rock".to_string(),
            "pop".to_string(),
        ];
        assert_eq!(primary_genre(&genres), Some("Indie/Alternative"));
    }

    #[test]
    fn primary_genre_returns_none_for_empty_input() {
        assert_eq!(primary_genre(&[]), None);
    }

    #[test]
    fn primary_genre_uses_input_order_to_break_ties() {
        let first = vec!["pop".to_string(), "rock".to_string()];
        let second = vec!["rock".to_string(), "pop".to_string()];
        assert_eq!(primary_genre(&first), Some("Rock"));
        assert_eq!(primary_genre(&second), Some("Pop"));
    }

    #[test]
    fn primary_genre_collapses_unmapped_genres_into_other() {
        let genres = vec![
            "field recordings".to_string(),
            "spoken word".to_string(),
            "pop".to_string(),
        ];
        assert_eq!(primary_genre(&genres), Some("Other"));
    }

    #[test]
    fn primary_genre_handles_one_genre() {
        assert_eq!(primary_genre(&["jazz".to_string()]), Some("Jazz"));
    }

    #[test]
    fn parse_release_year_accepts_boundary_years() {
        assert_eq!(parse_release_year("1000"), Some(1000));
        assert_eq!(parse_release_year("3000"), Some(3000));
        assert_eq!(parse_release_year("0999"), None);
        assert_eq!(parse_release_year("3001"), None);
    }

    #[test]
    fn parse_release_year_rejects_malformed_prefixes() {
        assert_eq!(parse_release_year(""), None);
        assert_eq!(parse_release_year("19"), None);
        assert_eq!(parse_release_year("abcd"), None);
        assert_eq!(parse_release_year(" 1981"), None);
        assert_eq!(parse_release_year("+198"), None);
    }

    #[test]
    fn parse_release_year_ignores_the_time_part() {
        assert_eq!(parse_release_year("2024-02-29T12:00:00Z"), Some(2024));
    }

    #[test]
    fn decade_from_year_rounds_down_to_the_decade() {
        assert_eq!(decade_from_year(1999), 1990);
        assert_eq!(decade_from_year(2000), 2000);
        assert_eq!(decade_from_year(2009), 2000);
        assert_eq!(decade_from_year(2010), 2010);
        assert_eq!(decade_from_year(0), 0);
    }
}
