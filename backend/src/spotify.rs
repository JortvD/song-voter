use anyhow::Context;
use serde::Deserialize;

pub const DEFAULT_API_BASE: &str = "https://api.spotify.com/v1";
pub const DEFAULT_ACCOUNTS_BASE: &str = "https://accounts.spotify.com/api";
pub const DEFAULT_AUTHORIZE_URL: &str = "https://accounts.spotify.com/authorize";
pub const DEFAULT_EMBED_BASE: &str = "https://open.spotify.com/embed/track";

/// Scopes needed to read playlists owned by, or shared with, the admin account.
pub const OAUTH_SCOPES: &str =
    "playlist-read-private playlist-read-collaborative user-read-private";

const PLAYLIST_FIELDS: &str = "id,name,description,images(url,width),owner(display_name)";
const PAGE_SIZE: u32 = 50;

/// A fully resolved playlist ready to be persisted.
#[derive(Debug, Clone)]
pub struct ImportedPlaylist {
    pub spotify_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_name: Option<String>,
    pub tracks: Vec<ImportedTrack>,
    pub skipped_tracks: usize,
}

/// Playlist metadata without fetching its items. Used when scheduling a
/// playlist before its song list is loaded.
#[derive(Debug, Clone)]
pub struct ImportedPlaylistInfo {
    pub spotify_id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImportedTrack {
    pub spotify_id: String,
    pub title: String,
    pub artists: String,
    pub artist_refs: Vec<ImportedArtistRef>,
    pub album: Option<String>,
    pub album_id: Option<String>,
    pub album_image_url: Option<String>,
    pub album_type: Option<String>,
    pub total_tracks: Option<i32>,
    pub release_date: Option<String>,
    pub release_date_precision: Option<String>,
    pub release_year: Option<i32>,
    pub preview_url: Option<String>,
    pub duration_ms: Option<i32>,
    pub disc_number: Option<i32>,
    pub track_number: Option<i32>,
    pub position: i32,
}

#[derive(Debug, Clone)]
pub struct ImportedArtistRef {
    pub spotify_id: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ImportedArtist {
    pub spotify_id: String,
    pub name: String,
    pub genres: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SpotifyError {
    #[error("playlist not found or not accessible")]
    NotFound,
    #[error("Spotify denied access to this playlist")]
    Forbidden,
    #[error("Spotify is rate limiting requests")]
    RateLimited,
    #[error("Spotify rejected the configured app credentials")]
    Unauthorized,
    #[error("Spotify connection has expired and must be renewed")]
    Reconnect,
    #[error("Spotify API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct TokenSet {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: i64,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpotifyUser {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Clone)]
pub struct SpotifyClient {
    http: reqwest::Client,
    client_id: String,
    client_secret: String,
    api_base: String,
    accounts_base: String,
    authorize_url: String,
    embed_base: String,
}

impl SpotifyClient {
    pub fn with_bases(
        client_id: String,
        client_secret: String,
        api_base: String,
        accounts_base: String,
    ) -> Self {
        Self {
            http: reqwest::Client::new(),
            client_id,
            client_secret,
            api_base,
            accounts_base,
            authorize_url: DEFAULT_AUTHORIZE_URL.to_string(),
            embed_base: DEFAULT_EMBED_BASE.to_string(),
        }
    }

    pub fn with_authorize_url(mut self, authorize_url: String) -> Self {
        self.authorize_url = authorize_url;
        self
    }

    pub fn with_embed_base(mut self, embed_base: String) -> Self {
        self.embed_base = embed_base;
        self
    }

    pub fn authorize_url(&self, redirect_uri: &str, state: &str) -> Result<String, SpotifyError> {
        let mut url =
            reqwest::Url::parse(&self.authorize_url).context("invalid Spotify authorize URL")?;

        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("scope", OAUTH_SCOPES)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("state", state)
            .append_pair("show_dialog", "true");

        Ok(url.to_string())
    }

    pub async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenSet, SpotifyError> {
        self.token_request(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .await
    }

    pub async fn refresh_user_token(&self, refresh_token: &str) -> Result<TokenSet, SpotifyError> {
        self.token_request(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .await
    }

    pub async fn current_user(&self, token: &str) -> Result<SpotifyUser, SpotifyError> {
        self.get_json(token, &format!("{}/me", self.api_base), vec![])
            .await
    }

    /// Spotify's Web API often returns a null `preview_url` for Development
    /// Mode apps. The public embed page still carries the 30-second preview
    /// MP3 URL, so we read it from there as a fallback.
    pub async fn fetch_preview_url(&self, spotify_id: &str) -> Option<String> {
        let url = format!("{}/{}", self.embed_base.trim_end_matches('/'), spotify_id);

        let response = self
            .http
            .get(&url)
            .header(
                "user-agent",
                "Mozilla/5.0 (compatible; PlaylistVoter/0.1; +https://vote.jortvd.nl)",
            )
            .send()
            .await
            .ok()?;

        if !response.status().is_success() {
            tracing::debug!(
                %spotify_id,
                status = %response.status(),
                "Spotify embed preview lookup failed"
            );
            return None;
        }

        let html = response.text().await.ok()?;
        let preview = extract_preview_url(&html);
        if preview.is_none() {
            tracing::debug!(%spotify_id, "no preview URL found on Spotify embed page");
        }
        preview
    }

    /// Used by tests and by extended-quota apps that can still read public
    /// playlists without a user token.
    pub async fn fetch_playlist(
        &self,
        playlist_id: &str,
    ) -> Result<ImportedPlaylist, SpotifyError> {
        let token = self.client_credentials_token().await?;
        self.fetch_playlist_with_token(playlist_id, &token).await
    }

    pub async fn fetch_playlist_info(
        &self,
        playlist_id: &str,
    ) -> Result<ImportedPlaylistInfo, SpotifyError> {
        let token = self.client_credentials_token().await?;
        self.fetch_playlist_info_with_token(playlist_id, &token)
            .await
    }

    pub async fn fetch_playlist_info_with_token(
        &self,
        playlist_id: &str,
        token: &str,
    ) -> Result<ImportedPlaylistInfo, SpotifyError> {
        let metadata: PlaylistMetadata = self
            .get_json(
                token,
                &format!("{}/playlists/{playlist_id}", self.api_base),
                vec![("fields", PLAYLIST_FIELDS.to_string())],
            )
            .await?;

        Ok(ImportedPlaylistInfo {
            spotify_id: metadata.id,
            name: metadata.name,
            description: metadata.description,
            image_url: largest_image_url(&metadata.images),
            owner_name: metadata.owner.display_name,
        })
    }

    /// Fetch artist genres in batches of 50, the Spotify API maximum.
    pub async fn fetch_artists(
        &self,
        token: &str,
        spotify_ids: &[String],
    ) -> Result<Vec<ImportedArtist>, SpotifyError> {
        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for id in spotify_ids {
            if !id.is_empty() && seen.insert(id.clone()) {
                unique.push(id.clone());
            }
        }

        let mut artists = Vec::new();
        for chunk in unique.chunks(50) {
            let page: ArtistList = self
                .get_json(
                    token,
                    &format!("{}/artists", self.api_base),
                    vec![("ids", chunk.join(","))],
                )
                .await?;

            for artist in page.artists {
                let Some(id) = artist.id.filter(|id| !id.is_empty()) else {
                    continue;
                };
                artists.push(ImportedArtist {
                    spotify_id: id,
                    name: artist.name,
                    genres: artist.genres,
                });
            }
        }

        tracing::debug!(
            requested = spotify_ids.len(),
            found = artists.len(),
            "fetched Spotify artists"
        );
        Ok(artists)
    }

    /// Fetches playlist metadata and every item using a user access token.
    pub async fn fetch_playlist_with_token(
        &self,
        playlist_id: &str,
        token: &str,
    ) -> Result<ImportedPlaylist, SpotifyError> {
        let metadata: PlaylistMetadata = self
            .get_json(
                token,
                &format!("{}/playlists/{playlist_id}", self.api_base),
                vec![("fields", PLAYLIST_FIELDS.to_string())],
            )
            .await?;

        tracing::debug!(
            playlist_id = %metadata.id,
            name = %metadata.name,
            "fetched Spotify playlist metadata"
        );

        let mut tracks = Vec::new();
        let mut items_seen = 0usize;
        let mut offset = 0u32;

        loop {
            let page: TracksPage = self
                .get_json(
                    token,
                    &format!("{}/playlists/{playlist_id}/items", self.api_base),
                    vec![
                        ("limit", PAGE_SIZE.to_string()),
                        ("offset", offset.to_string()),
                    ],
                )
                .await?;

            let total = page.total;
            let page_len = page.items.len();
            tracing::debug!(
                offset,
                total,
                received = page_len,
                "fetched Spotify playlist page"
            );

            for (index, item) in page.items.into_iter().enumerate() {
                let position = offset as usize + index;
                match item
                    .take_track()
                    .and_then(|track| track.into_imported(position))
                {
                    Some(track) => tracks.push(track),
                    None => {
                        tracing::debug!(position, "skipping unavailable, local or non-track item");
                    }
                }
            }

            items_seen += page_len;
            offset += page_len as u32;

            if page.next.is_none() || page_len == 0 || offset >= total {
                break;
            }
        }

        // A song can legally appear twice in a playlist; keep the first occurrence.
        let mut seen = std::collections::HashSet::new();
        tracks.retain(|track| seen.insert(track.spotify_id.clone()));
        let skipped_tracks = items_seen.saturating_sub(tracks.len());

        tracing::debug!(
            playlist_id = %metadata.id,
            tracks = tracks.len(),
            skipped = skipped_tracks,
            "Spotify playlist resolved"
        );

        Ok(ImportedPlaylist {
            spotify_id: metadata.id,
            name: metadata.name,
            description: metadata.description,
            image_url: largest_image_url(&metadata.images),
            owner_name: metadata.owner.display_name,
            tracks,
            skipped_tracks,
        })
    }

    pub(crate) async fn client_credentials_token(&self) -> Result<String, SpotifyError> {
        let tokens = self
            .token_request(&[("grant_type", "client_credentials")])
            .await?;

        Ok(tokens.access_token)
    }

    async fn token_request(&self, form: &[(&str, &str)]) -> Result<TokenSet, SpotifyError> {
        let response = self
            .http
            .post(format!("{}/token", self.accounts_base))
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(form)
            .send()
            .await
            .context("failed to reach Spotify accounts API")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("failed to read Spotify token response")?;

        if !status.is_success() {
            tracing::warn!(%status, "Spotify token request rejected");
            let lowered = body.to_ascii_lowercase();
            return Err(match status.as_u16() {
                400 if lowered.contains("invalid_grant") => SpotifyError::Reconnect,
                401 | 403 => SpotifyError::Unauthorized,
                429 => SpotifyError::RateLimited,
                code => SpotifyError::Api {
                    status: code,
                    message: truncate_body(&body),
                },
            });
        }

        let token: TokenResponse =
            serde_json::from_str(&body).context("failed to parse Spotify token response")?;

        tracing::debug!(
            expires_in = token.expires_in,
            has_refresh_token = token.refresh_token.is_some(),
            "obtained Spotify token"
        );

        Ok(TokenSet {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_in: token.expires_in,
            scope: token.scope,
        })
    }

    async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        token: &str,
        url: &str,
        query: Vec<(&str, String)>,
    ) -> Result<T, SpotifyError> {
        const MAX_ATTEMPTS: u32 = 3;

        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .http
                .get(url)
                .bearer_auth(token)
                .query(&query)
                .send()
                .await
                .context("failed to reach Spotify API")?;

            let status = response.status();

            if status.as_u16() == 429 {
                let wait_seconds = retry_after_seconds(&response).clamp(1, 30);
                tracing::warn!(attempt, wait_seconds, "Spotify rate limit hit, backing off");
                tokio::time::sleep(std::time::Duration::from_secs(wait_seconds)).await;
                continue;
            }

            if status.is_server_error() && attempt < MAX_ATTEMPTS {
                let wait_seconds = attempt as u64 * 2;
                tracing::warn!(attempt, %status, wait_seconds, "Spotify server error, retrying");
                tokio::time::sleep(std::time::Duration::from_secs(wait_seconds)).await;
                continue;
            }

            let body = response
                .text()
                .await
                .context("failed to read Spotify response")?;

            if !status.is_success() {
                tracing::warn!(%status, body = %truncate_body(&body), "Spotify API request failed");
                return Err(match status.as_u16() {
                    401 => SpotifyError::Unauthorized,
                    403 => SpotifyError::Forbidden,
                    404 => SpotifyError::NotFound,
                    429 => SpotifyError::RateLimited,
                    code => SpotifyError::Api {
                        status: code,
                        message: truncate_body(&body),
                    },
                });
            }

            return Ok(serde_json::from_str(&body).context("failed to parse Spotify response")?);
        }

        Err(SpotifyError::RateLimited)
    }
}

/// Accepts a full Spotify URL, a `spotify:playlist:` URI, or a bare 22-char ID.
pub fn parse_playlist_id(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let candidate = if let Some(rest) = input.strip_prefix("spotify:playlist:") {
        rest.split(['?', '/']).next().unwrap_or(rest)
    } else if let Some(index) = input.find("/playlist/") {
        let rest = &input[index + "/playlist/".len()..];
        rest.split(['?', '#', '/']).next().unwrap_or(rest)
    } else {
        input
    };

    let candidate = candidate.trim();
    if candidate.len() == 22
        && candidate
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        Some(candidate.to_string())
    } else {
        None
    }
}

/// Pulls the 30-second preview MP3 URL out of a Spotify embed page.
pub fn extract_preview_url(html: &str) -> Option<String> {
    const MARKER: &str = "https://p.scdn.co/mp3-preview/";

    // The URL is sometimes embedded in JSON with escaped slashes.
    let normalized = html.replace("\\/", "/");
    let start = normalized.find(MARKER)?;
    let rest = &normalized[start..];
    let end = rest
        .find(['"', '\'', '<', ' ', '\\', '\n', '\r'])
        .unwrap_or(rest.len());
    let url = &rest[..end];

    if url.len() > MARKER.len() {
        Some(url.to_string())
    } else {
        None
    }
}

fn largest_image_url(images: &[Image]) -> Option<String> {
    images
        .iter()
        .max_by_key(|image| image.width.unwrap_or(0))
        .map(|image| image.url.clone())
}

fn retry_after_seconds(response: &reqwest::Response) -> u64 {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(2)
}

fn truncate_body(body: &str) -> String {
    body.chars().take(300).collect()
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: i64,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaylistMetadata {
    id: String,
    name: String,
    description: Option<String>,
    #[serde(default)]
    images: Vec<Image>,
    owner: Owner,
}

#[derive(Debug, Deserialize)]
struct Owner {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Image {
    url: String,
    width: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TracksPage {
    #[serde(default)]
    items: Vec<TrackItem>,
    next: Option<String>,
    #[serde(default)]
    total: u32,
}

#[derive(Debug, Deserialize)]
struct TrackItem {
    /// New name for this field after Spotify's February 2026 API migration.
    item: Option<TrackObject>,
    /// Deprecated, but still present for a while.
    track: Option<TrackObject>,
}

impl TrackItem {
    fn take_track(self) -> Option<TrackObject> {
        self.item.or(self.track)
    }
}

#[derive(Debug, Deserialize)]
struct TrackObject {
    id: Option<String>,
    name: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    is_local: bool,
    #[serde(default)]
    artists: Vec<ArtistObject>,
    #[serde(default)]
    album: Option<AlbumObject>,
    preview_url: Option<String>,
    duration_ms: Option<i64>,
    disc_number: Option<i32>,
    track_number: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ArtistObject {
    id: Option<String>,
    name: String,
    #[serde(default)]
    genres: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AlbumObject {
    id: Option<String>,
    name: String,
    album_type: Option<String>,
    total_tracks: Option<i32>,
    release_date: Option<String>,
    release_date_precision: Option<String>,
    #[serde(default)]
    images: Vec<Image>,
}

#[derive(Debug, Deserialize)]
struct ArtistList {
    #[serde(default)]
    artists: Vec<ArtistObject>,
}

impl TrackObject {
    fn into_imported(self, position: usize) -> Option<ImportedTrack> {
        if self.is_local || (!self.kind.is_empty() && self.kind != "track") {
            return None;
        }

        let spotify_id = self.id.filter(|id| !id.is_empty())?;
        let title = self.name.trim();
        if title.is_empty() {
            return None;
        }

        let artist_refs = self
            .artists
            .iter()
            .map(|artist| ImportedArtistRef {
                spotify_id: artist.id.clone().filter(|id| !id.is_empty()),
                name: artist.name.clone(),
            })
            .collect::<Vec<_>>();

        let artists = if artist_refs.is_empty() {
            "Unknown artist".to_string()
        } else {
            artist_refs
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };

        let album_id = self.album.as_ref().and_then(|album| album.id.clone());
        let album = self.album.as_ref().map(|album| album.name.clone());
        let album_type = self
            .album
            .as_ref()
            .and_then(|album| album.album_type.clone());
        let total_tracks = self.album.as_ref().and_then(|album| album.total_tracks);
        let release_date = self
            .album
            .as_ref()
            .and_then(|album| album.release_date.clone());
        let release_date_precision = self
            .album
            .as_ref()
            .and_then(|album| album.release_date_precision.clone());
        let release_year = release_date
            .as_deref()
            .and_then(crate::metadata::parse_release_year);
        let album_image_url = self
            .album
            .as_ref()
            .and_then(|album| largest_image_url(&album.images));

        Some(ImportedTrack {
            spotify_id,
            title: title.to_string(),
            artists,
            artist_refs,
            album,
            album_id,
            album_image_url,
            album_type,
            total_tracks,
            release_date,
            release_date_precision,
            release_year,
            preview_url: self.preview_url.filter(|url| !url.trim().is_empty()),
            duration_ms: self.duration_ms.and_then(|value| i32::try_from(value).ok()),
            disc_number: self.disc_number,
            track_number: self.track_number,
            position: i32::try_from(position).unwrap_or(i32::MAX),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parses_playlist_ids_from_all_supported_forms() {
        let id = "37i9dQZF1DXcBWIGoYBM5M";
        assert_eq!(parse_playlist_id(id).as_deref(), Some(id), "bare id");
        assert_eq!(
            parse_playlist_id(&format!("https://open.spotify.com/playlist/{id}?si=abc")).as_deref(),
            Some(id),
            "url with query"
        );
        assert_eq!(
            parse_playlist_id(&format!("spotify:playlist:{id}")).as_deref(),
            Some(id),
            "uri"
        );
        assert_eq!(parse_playlist_id("https://example.com/nope"), None);
        assert_eq!(parse_playlist_id("too-short"), None);
    }

    #[test]
    fn authorize_url_contains_scopes_and_state() {
        let client = SpotifyClient::with_bases(
            "client-id".to_string(),
            "secret".to_string(),
            DEFAULT_API_BASE.to_string(),
            DEFAULT_ACCOUNTS_BASE.to_string(),
        );

        let url = client
            .authorize_url("https://example.com/callback", "state-123")
            .unwrap();
        let parsed = reqwest::Url::parse(&url).unwrap();
        let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();

        assert_eq!(params.get("response_type").unwrap(), "code");
        assert_eq!(params.get("client_id").unwrap(), "client-id");
        assert_eq!(params.get("state").unwrap(), "state-123");
        assert_eq!(
            params.get("redirect_uri").unwrap(),
            "https://example.com/callback"
        );
        assert!(params
            .get("scope")
            .unwrap()
            .contains("playlist-read-private"));
    }

    #[test]
    fn extracts_preview_urls_from_embed_pages() {
        let html = r#"<script>{"preview_url":"https://p.scdn.co/mp3-preview/abc123"}</script>"#;
        assert_eq!(
            extract_preview_url(html).as_deref(),
            Some("https://p.scdn.co/mp3-preview/abc123")
        );

        let escaped = r#"{"url":"https:\/\/p.scdn.co\/mp3-preview\/def456"}"#;
        assert_eq!(
            extract_preview_url(escaped).as_deref(),
            Some("https://p.scdn.co/mp3-preview/def456")
        );

        assert_eq!(extract_preview_url("no preview here"), None);
    }

    #[tokio::test]
    async fn fetches_and_normalises_a_playlist() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "test-token",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1/playlists/testid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "testid",
                "name": "Test Chart",
                "description": "A test playlist",
                "images": [
                    {"url": "small.jpg", "width": 64},
                    {"url": "large.jpg", "width": 640}
                ],
                "owner": {"display_name": "jort"}
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1/playlists/testid/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"item": {"id": "t1", "name": "Song One", "type": "track", "is_local": false,
                        "duration_ms": 210000,
                        "preview_url": "https://p.scdn.co/mp3-preview/test",
                        "artists": [{"name": "Artist A"}, {"name": "Artist B"}],
                        "album": {"name": "Album One", "images": [{"url": "art.jpg", "width": 300}]}}},
                    {"track": {"id": "t1", "name": "Duplicate", "type": "track", "is_local": false,
                        "duration_ms": 1000, "artists": [], "album": {"name": "Album One", "images": []}}},
                    {"item": {"id": "", "name": "Episode", "type": "episode", "is_local": false,
                        "duration_ms": 1000, "artists": [], "album": null}},
                    {"item": {"id": "local", "name": "Local File", "type": "track", "is_local": true,
                        "duration_ms": 1000, "artists": [], "album": null}},
                    {"item": null},
                    {"track": {"id": "t2", "name": "Song Two", "type": "track", "is_local": false,
                        "duration_ms": 180000, "artists": [{"name": "Artist C"}],
                        "album": {"name": "Album Two", "images": [{"url": "art2.jpg", "width": 640}]}}}
                ],
                "next": null,
                "total": 6
            })))
            .mount(&server)
            .await;

        let client = SpotifyClient::with_bases(
            "id".to_string(),
            "secret".to_string(),
            format!("{}/v1", server.uri()),
            format!("{}/api", server.uri()),
        );

        let playlist = client.fetch_playlist("testid").await.unwrap();

        assert_eq!(playlist.name, "Test Chart");
        assert_eq!(playlist.owner_name.as_deref(), Some("jort"));
        assert_eq!(playlist.image_url.as_deref(), Some("large.jpg"));
        assert_eq!(playlist.tracks.len(), 2);
        assert_eq!(playlist.skipped_tracks, 4);

        let first = &playlist.tracks[0];
        assert_eq!(first.title, "Song One");
        assert_eq!(first.artists, "Artist A, Artist B");
        assert_eq!(first.album.as_deref(), Some("Album One"));
        assert_eq!(first.album_image_url.as_deref(), Some("art.jpg"));
        assert_eq!(
            first.preview_url.as_deref(),
            Some("https://p.scdn.co/mp3-preview/test")
        );
        assert_eq!(first.duration_ms, Some(210000));
        assert_eq!(first.position, 0);
        assert_eq!(playlist.tracks[1].position, 5);
    }

    #[tokio::test]
    async fn private_playlist_maps_to_a_forbidden_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "test-token",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/v1/playlists/privateid"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "error": {"status": 403, "message": "Forbidden"}
            })))
            .mount(&server)
            .await;

        let client = SpotifyClient::with_bases(
            "id".to_string(),
            "secret".to_string(),
            format!("{}/v1", server.uri()),
            format!("{}/api", server.uri()),
        );

        let result = client.fetch_playlist("privateid").await;
        assert!(
            matches!(result, Err(SpotifyError::Forbidden)),
            "expected Forbidden, got {result:?}"
        );
    }

    #[tokio::test]
    async fn exchanges_an_authorization_code() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "user-token",
                "refresh_token": "refresh-token",
                "expires_in": 3600,
                "scope": OAUTH_SCOPES,
                "token_type": "Bearer"
            })))
            .mount(&server)
            .await;

        let client = SpotifyClient::with_bases(
            "id".to_string(),
            "secret".to_string(),
            format!("{}/v1", server.uri()),
            format!("{}/api", server.uri()),
        );

        let tokens = client
            .exchange_code("code-123", "https://example.com/callback")
            .await
            .unwrap();

        assert_eq!(tokens.access_token, "user-token");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-token"));
        assert_eq!(tokens.expires_in, 3600);
    }

    fn test_client(server: &MockServer) -> SpotifyClient {
        SpotifyClient::with_bases(
            "id".to_string(),
            "secret".to_string(),
            format!("{}/v1", server.uri()),
            format!("{}/api", server.uri()),
        )
    }

    async fn mock_token(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "test-token",
                "expires_in": 3600,
                "token_type": "Bearer"
            })))
            .mount(server)
            .await;
    }

    async fn mock_metadata(server: &MockServer, id: &str) {
        Mock::given(method("GET"))
            .and(path(format!("/v1/playlists/{id}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": id,
                "name": "Playlist",
                "description": null,
                "images": [],
                "owner": {"display_name": null}
            })))
            .mount(server)
            .await;
    }

    #[test]
    fn parses_playlist_id_from_uri_with_query() {
        let id = "37i9dQZF1DXcBWIGoYBM5M";
        assert_eq!(
            parse_playlist_id(&format!("spotify:playlist:{id}?si=x")).as_deref(),
            Some(id)
        );
    }

    #[test]
    fn parses_playlist_id_from_url_with_trailing_slash_and_fragment() {
        let id = "37i9dQZF1DXcBWIGoYBM5M";
        assert_eq!(
            parse_playlist_id(&format!(
                "https://open.spotify.com/playlist/{id}/?si=abc#frag"
            ))
            .as_deref(),
            Some(id)
        );
    }

    #[test]
    fn parse_playlist_id_trims_surrounding_whitespace() {
        let id = "37i9dQZF1DXcBWIGoYBM5M";
        assert_eq!(parse_playlist_id(&format!("  {id}  ")).as_deref(), Some(id));
    }

    #[test]
    fn parse_playlist_id_rejects_wrong_length_and_symbols() {
        assert_eq!(parse_playlist_id(""), None);
        assert_eq!(parse_playlist_id("   "), None);
        assert_eq!(parse_playlist_id("123456789012345678901"), None);
        assert_eq!(parse_playlist_id("123456789012345678901!"), None);
        assert_eq!(parse_playlist_id("12345678901234567890123"), None);
    }

    #[test]
    fn parse_playlist_id_accepts_mixed_case_alphanumerics() {
        let id = "AbCdEfGhIjKlMnOpQrStUv";
        assert_eq!(parse_playlist_id(id).as_deref(), Some(id));
    }

    #[test]
    fn extract_preview_url_stops_at_common_delimiters() {
        assert_eq!(
            extract_preview_url("https://p.scdn.co/mp3-preview/abc'def").as_deref(),
            Some("https://p.scdn.co/mp3-preview/abc")
        );
        assert_eq!(
            extract_preview_url("https://p.scdn.co/mp3-preview/abc<tag>").as_deref(),
            Some("https://p.scdn.co/mp3-preview/abc")
        );
        assert_eq!(
            extract_preview_url("https://p.scdn.co/mp3-preview/abc def").as_deref(),
            Some("https://p.scdn.co/mp3-preview/abc")
        );
    }

    #[test]
    fn extract_preview_url_requires_a_path_after_the_marker() {
        assert_eq!(extract_preview_url("https://p.scdn.co/mp3-preview/"), None);
        assert_eq!(
            extract_preview_url("https://p.scdn.co/mp3-preview/\"quote\""),
            None
        );
    }

    #[test]
    fn extract_preview_url_accepts_nothing_but_the_marker() {
        // A path segment of one character is still a usable URL.
        assert_eq!(
            extract_preview_url("https://p.scdn.co/mp3-preview/a").as_deref(),
            Some("https://p.scdn.co/mp3-preview/a")
        );
    }

    #[test]
    fn largest_image_prefers_the_widest_image() {
        let images = vec![
            Image {
                url: "small".into(),
                width: Some(64),
            },
            Image {
                url: "large".into(),
                width: Some(640),
            },
            Image {
                url: "medium".into(),
                width: Some(300),
            },
        ];
        assert_eq!(largest_image_url(&images).as_deref(), Some("large"));
    }

    #[test]
    fn largest_image_handles_unknown_widths_and_empty_lists() {
        let images = vec![
            Image {
                url: "unknown".into(),
                width: None,
            },
            Image {
                url: "known".into(),
                width: Some(1),
            },
        ];
        assert_eq!(largest_image_url(&images).as_deref(), Some("known"));
        assert_eq!(largest_image_url(&[]), None);
    }

    #[test]
    fn largest_image_keeps_the_last_on_equal_widths() {
        let images = vec![
            Image {
                url: "first".into(),
                width: Some(100),
            },
            Image {
                url: "second".into(),
                width: Some(100),
            },
        ];
        assert_eq!(largest_image_url(&images).as_deref(), Some("second"));
    }

    #[test]
    fn truncate_body_respects_the_character_limit() {
        assert_eq!(truncate_body("abc"), "abc");
        assert_eq!(truncate_body(&"a".repeat(301)).len(), 300);
        assert_eq!(truncate_body(&"é".repeat(400)).chars().count(), 300);
    }

    #[test]
    fn token_response_deserialises_defaults() {
        let token: TokenResponse = serde_json::from_value(json!({
            "access_token": "abc"
        }))
        .unwrap();
        assert_eq!(token.access_token, "abc");
        assert_eq!(token.expires_in, 0);
        assert_eq!(token.refresh_token, None);
        assert_eq!(token.scope, None);
    }

    #[test]
    fn track_item_prefers_the_new_item_field() {
        let item: TrackItem = serde_json::from_value(json!({
            "item": {"id": "new", "name": "New", "type": "track"},
            "track": {"id": "old", "name": "Old", "type": "track"}
        }))
        .unwrap();
        assert_eq!(item.take_track().unwrap().id.as_deref(), Some("new"));
    }

    #[test]
    fn track_item_falls_back_to_the_legacy_track_field() {
        let item: TrackItem = serde_json::from_value(json!({
            "track": {"id": "old", "name": "Old", "type": "track"}
        }))
        .unwrap();
        assert_eq!(item.take_track().unwrap().id.as_deref(), Some("old"));
    }

    #[test]
    fn track_item_with_both_fields_null_is_skipped() {
        let item: TrackItem = serde_json::from_value(json!({
            "item": null,
            "track": null
        }))
        .unwrap();
        assert!(item.take_track().is_none());
    }

    fn track_object(value: serde_json::Value) -> TrackObject {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn track_object_normalises_artists_album_and_metadata() {
        let track = track_object(json!({
            "id": "t1",
            "name": "  Song  ",
            "type": "track",
            "duration_ms": 123456,
            "disc_number": 1,
            "track_number": 4,
            "preview_url": " https://preview ",
            "artists": [
                {"id": "a1", "name": "One"},
                {"id": "", "name": "Two"}
            ],
            "album": {
                "id": "al1",
                "name": "Album",
                "album_type": "single",
                "total_tracks": 8,
                "release_date": "1999-05-01",
                "release_date_precision": "day",
                "images": [{"url": "art", "width": 300}]
            }
        }))
        .into_imported(7)
        .unwrap();

        assert_eq!(track.spotify_id, "t1");
        assert_eq!(track.title, "Song");
        assert_eq!(track.artists, "One, Two");
        assert_eq!(track.album.as_deref(), Some("Album"));
        assert_eq!(track.album_id.as_deref(), Some("al1"));
        assert_eq!(track.album_type.as_deref(), Some("single"));
        assert_eq!(track.total_tracks, Some(8));
        assert_eq!(track.release_year, Some(1999));
        assert_eq!(track.preview_url.as_deref(), Some(" https://preview "));
        assert_eq!(track.duration_ms, Some(123456));
        assert_eq!(track.position, 7);
    }

    #[test]
    fn track_object_rejects_local_non_track_and_missing_id_items() {
        for value in [
            json!({"id": "x", "name": "x", "type": "track", "is_local": true}),
            json!({"id": "x", "name": "x", "type": "episode"}),
            json!({"id": "", "name": "x", "type": "track"}),
            json!({"name": "x", "type": "track"}),
        ] {
            assert!(track_object(value).into_imported(0).is_none());
        }
    }

    #[test]
    fn track_object_rejects_blank_titles() {
        assert!(
            track_object(json!({"id": "x", "name": "   ", "type": "track"}))
                .into_imported(0)
                .is_none()
        );
    }

    #[test]
    fn track_object_defaults_unknown_artist_and_missing_album() {
        let track = track_object(json!({"id": "x", "name": "Song", "type": "track"}))
            .into_imported(0)
            .unwrap();
        assert_eq!(track.artists, "Unknown artist");
        assert!(track.artist_refs.is_empty());
        assert!(track.album.is_none());
        assert!(track.release_year.is_none());
        assert!(track.preview_url.is_none());
    }

    #[test]
    fn track_object_with_empty_type_is_still_a_track() {
        let track = track_object(json!({"id": "x", "name": "Song"}))
            .into_imported(0)
            .unwrap();
        assert_eq!(track.spotify_id, "x");
    }

    #[test]
    fn track_object_clamps_oversized_positions() {
        let track = track_object(json!({"id": "x", "name": "Song"}))
            .into_imported(usize::MAX)
            .unwrap();
        assert_eq!(track.position, i32::MAX);
    }

    #[test]
    fn track_object_drops_durations_that_do_not_fit_i32() {
        let track = track_object(json!({
            "id": "x", "name": "Song", "duration_ms": 9_000_000_000i64
        }))
        .into_imported(0)
        .unwrap();
        assert_eq!(track.duration_ms, None);
    }

    #[tokio::test]
    async fn fetches_metadata_without_track_items() {
        let server = MockServer::start().await;
        mock_token(&server).await;
        Mock::given(method("GET"))
            .and(path("/v1/playlists/abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "abc",
                "name": "Info Only",
                "description": "desc",
                "images": [{"url": "small", "width": 1}, {"url": "big", "width": 500}],
                "owner": {"display_name": "owner"}
            })))
            .mount(&server)
            .await;

        let info = test_client(&server)
            .fetch_playlist_info("abc")
            .await
            .unwrap();

        assert_eq!(info.spotify_id, "abc");
        assert_eq!(info.name, "Info Only");
        assert_eq!(info.description.as_deref(), Some("desc"));
        assert_eq!(info.image_url.as_deref(), Some("big"));
        assert_eq!(info.owner_name.as_deref(), Some("owner"));
    }

    #[tokio::test]
    async fn paginates_playlist_items_until_the_total_is_seen() {
        let server = MockServer::start().await;
        mock_token(&server).await;
        mock_metadata(&server, "paged").await;

        let first_page = (0..50)
            .map(|index| json!({"item": {"id": format!("t{index}"), "name": format!("S{index}"), "type": "track"}}))
            .collect::<Vec<_>>();
        let second_page = (50..60)
            .map(|index| json!({"item": {"id": format!("t{index}"), "name": format!("S{index}"), "type": "track"}}))
            .collect::<Vec<_>>();

        Mock::given(method("GET"))
            .and(path("/v1/playlists/paged/items"))
            .and(wiremock::matchers::query_param("offset", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": first_page, "next": "next-url", "total": 60
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/playlists/paged/items"))
            .and(wiremock::matchers::query_param("offset", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": second_page, "next": null, "total": 60
            })))
            .mount(&server)
            .await;

        let playlist = test_client(&server).fetch_playlist("paged").await.unwrap();

        assert_eq!(playlist.tracks.len(), 60);
        assert_eq!(playlist.tracks[0].position, 0);
        assert_eq!(playlist.tracks[59].position, 59);
        assert_eq!(playlist.skipped_tracks, 0);
    }

    #[tokio::test]
    async fn stops_paging_when_a_page_is_empty() {
        let server = MockServer::start().await;
        mock_token(&server).await;
        mock_metadata(&server, "empty").await;

        Mock::given(method("GET"))
            .and(path("/v1/playlists/empty/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [], "next": "still-next", "total": 10
            })))
            .mount(&server)
            .await;

        let playlist = test_client(&server).fetch_playlist("empty").await.unwrap();
        assert!(playlist.tracks.is_empty());
        assert_eq!(playlist.skipped_tracks, 0);
    }

    #[tokio::test]
    async fn counts_skipped_and_duplicate_tracks() {
        let server = MockServer::start().await;
        mock_token(&server).await;
        mock_metadata(&server, "dupes").await;

        Mock::given(method("GET"))
            .and(path("/v1/playlists/dupes/items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"item": {"id": "same", "name": "First", "type": "track"}},
                    {"item": {"id": "same", "name": "Second", "type": "track"}},
                    {"item": {"id": "episode", "name": "Episode", "type": "episode"}},
                    {"item": {"id": "local", "name": "Local", "type": "track", "is_local": true}}
                ],
                "next": null,
                "total": 4
            })))
            .mount(&server)
            .await;

        let playlist = test_client(&server).fetch_playlist("dupes").await.unwrap();

        assert_eq!(playlist.tracks.len(), 1);
        assert_eq!(playlist.tracks[0].title, "First");
        assert_eq!(playlist.skipped_tracks, 3);
    }

    #[tokio::test]
    async fn fetches_current_user() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "spotify-user",
                "display_name": "Jort"
            })))
            .mount(&server)
            .await;

        let user = test_client(&server).current_user("token").await.unwrap();
        assert_eq!(user.id, "spotify-user");
        assert_eq!(user.display_name.as_deref(), Some("Jort"));
    }

    #[tokio::test]
    async fn maps_api_status_codes_to_spotify_errors() {
        for (status, expected) in [
            (400, "Api"),
            (401, "Unauthorized"),
            (403, "Forbidden"),
            (404, "NotFound"),
        ] {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/v1/me"))
                .respond_with(ResponseTemplate::new(status).set_body_string("nope"))
                .mount(&server)
                .await;

            let error = test_client(&server)
                .current_user("token")
                .await
                .unwrap_err();
            match (status, expected) {
                (400, _) => assert!(matches!(error, SpotifyError::Api { status: 400, .. })),
                _ => assert_eq!(format!("{error:?}"), expected),
            }
        }
    }

    #[tokio::test]
    async fn rate_limited_api_responses_become_rate_limited_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/me"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
            .mount(&server)
            .await;

        // Retries are clamped to one second per attempt, so keep this to the
        // three attempts the client makes.
        let error = test_client(&server)
            .current_user("token")
            .await
            .unwrap_err();
        assert!(matches!(error, SpotifyError::RateLimited));
    }

    #[tokio::test]
    async fn maps_unknown_api_errors_with_status_and_truncated_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/me"))
            .respond_with(ResponseTemplate::new(418).set_body_string("x".repeat(400)))
            .mount(&server)
            .await;

        let error = test_client(&server)
            .current_user("token")
            .await
            .unwrap_err();
        match error {
            SpotifyError::Api { status, message } => {
                assert_eq!(status, 418);
                assert_eq!(message.len(), 300);
            }
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn token_reconnect_maps_invalid_grant() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid_grant"))
            .mount(&server)
            .await;

        let error = test_client(&server)
            .exchange_code("bad", "https://example.com/cb")
            .await
            .unwrap_err();
        assert!(matches!(error, SpotifyError::Reconnect));
    }

    #[tokio::test]
    async fn token_unauthorized_and_rate_limited_are_mapped() {
        for (status, expected) in [(401, "Unauthorized"), (429, "RateLimited")] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/api/token"))
                .respond_with(ResponseTemplate::new(status).set_body_string("no"))
                .mount(&server)
                .await;

            let error = test_client(&server)
                .exchange_code("code", "https://example.com/cb")
                .await
                .unwrap_err();
            assert_eq!(format!("{error:?}"), expected);
        }
    }

    #[tokio::test]
    async fn refreshes_a_user_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new-access",
                "expires_in": 1800
            })))
            .mount(&server)
            .await;

        let tokens = test_client(&server)
            .refresh_user_token("refresh")
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "new-access");
        assert_eq!(tokens.refresh_token, None);
        assert_eq!(tokens.expires_in, 1800);
    }

    #[tokio::test]
    async fn fetch_artists_deduplicates_and_skips_empty_ids() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/artists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "artists": [
                    {"id": "a1", "name": "One", "genres": ["pop"]},
                    {"id": "", "name": "Empty", "genres": []},
                    {"name": "Missing", "genres": []}
                ]
            })))
            .mount(&server)
            .await;

        let artists = test_client(&server)
            .fetch_artists("token", &["a1".into(), "a1".into(), "".into()])
            .await
            .unwrap();

        assert_eq!(artists.len(), 1);
        assert_eq!(artists[0].spotify_id, "a1");
        assert_eq!(artists[0].genres, vec!["pop".to_string()]);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn fetch_artists_batches_at_fifty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/artists"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"artists": []})))
            .mount(&server)
            .await;

        let ids = (0..51).map(|index| format!("a{index}")).collect::<Vec<_>>();
        let artists = test_client(&server)
            .fetch_artists("token", &ids)
            .await
            .unwrap();

        assert!(artists.is_empty());
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn preview_lookup_reads_the_embed_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/embed/track/abc"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<script>{"preview_url":"https://p.scdn.co/mp3-preview/from-embed"}</script>"#,
            ))
            .mount(&server)
            .await;

        let client = test_client(&server).with_embed_base(format!("{}/embed/track", server.uri()));
        assert_eq!(
            client.fetch_preview_url("abc").await.as_deref(),
            Some("https://p.scdn.co/mp3-preview/from-embed")
        );
    }

    #[tokio::test]
    async fn preview_lookup_returns_none_for_missing_or_failed_pages() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/embed/track/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/embed/track/blank"))
            .respond_with(ResponseTemplate::new(200).set_body_string("nothing here"))
            .mount(&server)
            .await;

        let client = test_client(&server).with_embed_base(format!("{}/embed/track", server.uri()));
        assert_eq!(client.fetch_preview_url("missing").await, None);
        assert_eq!(client.fetch_preview_url("blank").await, None);
    }

    #[test]
    fn authorize_url_rejects_an_invalid_base_url() {
        let client = SpotifyClient::with_bases(
            "id".into(),
            "secret".into(),
            DEFAULT_API_BASE.into(),
            DEFAULT_ACCOUNTS_BASE.into(),
        )
        .with_authorize_url("://not-a-url".into());
        assert!(client
            .authorize_url("https://example.com/cb", "state")
            .is_err());
    }
}
