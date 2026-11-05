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

#[derive(Debug, Clone)]
pub struct ImportedTrack {
    pub spotify_id: String,
    pub title: String,
    pub artists: String,
    pub album: Option<String>,
    pub album_image_url: Option<String>,
    pub preview_url: Option<String>,
    pub duration_ms: Option<i32>,
    pub position: i32,
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

    pub fn authorize_url(
        &self,
        redirect_uri: &str,
        state: &str,
    ) -> Result<String, SpotifyError> {
        let mut url = reqwest::Url::parse(&self.authorize_url)
            .context("invalid Spotify authorize URL")?;

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

    pub async fn refresh_user_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenSet, SpotifyError> {
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
        let url = format!(
            "{}/{}",
            self.embed_base.trim_end_matches('/'),
            spotify_id
        );

        let response = self
            .http
            .get(&url)
            .header(
                "user-agent",
                "Mozilla/5.0 (compatible; ULLTop2000/0.1; +https://vote.jortvd.nl)",
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
            tracing::debug!(offset, total, received = page_len, "fetched Spotify playlist page");

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

    async fn client_credentials_token(&self) -> Result<String, SpotifyError> {
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
                tracing::warn!(
                    attempt,
                    wait_seconds,
                    "Spotify rate limit hit, backing off"
                );
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

            return Ok(serde_json::from_str(&body)
                .context("failed to parse Spotify response")?);
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
    if candidate.len() == 22 && candidate.chars().all(|character| character.is_ascii_alphanumeric())
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
        .find(|character: char| {
            matches!(character, '"' | '\'' | '<' | ' ' | '\\' | '\n' | '\r')
        })
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
}

#[derive(Debug, Deserialize)]
struct ArtistObject {
    name: String,
}

#[derive(Debug, Deserialize)]
struct AlbumObject {
    name: String,
    #[serde(default)]
    images: Vec<Image>,
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

        let artists = if self.artists.is_empty() {
            "Unknown artist".to_string()
        } else {
            self.artists
                .into_iter()
                .map(|artist| artist.name)
                .collect::<Vec<_>>()
                .join(", ")
        };

        let album = self.album.as_ref().map(|album| album.name.clone());
        let album_image_url = self
            .album
            .as_ref()
            .and_then(|album| largest_image_url(&album.images));

        Some(ImportedTrack {
            spotify_id,
            title: title.to_string(),
            artists,
            album,
            album_image_url,
            preview_url: self
                .preview_url
                .filter(|url| !url.trim().is_empty()),
            duration_ms: self.duration_ms.and_then(|value| i32::try_from(value).ok()),
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
        assert_eq!(
            parse_playlist_id(id).as_deref(),
            Some(id),
            "bare id"
        );
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
}
