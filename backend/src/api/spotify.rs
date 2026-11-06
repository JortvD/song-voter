use anyhow::{bail, Context};
use axum::extract::{Query, State};
use axum::response::Redirect;
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::spotify::SpotifyError;
use crate::spotify_auth;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct SpotifyStatus {
    configured: bool,
    connected: bool,
    display_name: Option<String>,
    connected_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AuthorizeResponse {
    url: String,
}

#[derive(Debug, Serialize)]
pub struct DisconnectResponse {
    connected: bool,
}

pub async fn status(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<SpotifyStatus>, ApiError> {
    let configured = state.spotify.is_some() && state.spotify_redirect_uri.is_some();
    let connection = spotify_auth::status(&state.db)
        .await
        .map_err(|error| map_error(error, false))?;

    let (connected, display_name, connected_at, expires_at) = match connection {
        Some(connection) => (
            true,
            connection.display_name,
            Some(connection.connected_at),
            Some(connection.expires_at),
        ),
        None => (false, None, None, None),
    };

    Ok(Json(SpotifyStatus {
        configured,
        connected,
        display_name,
        connected_at,
        expires_at,
    }))
}

pub async fn authorize(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<AuthorizeResponse>, ApiError> {
    let client = state.spotify.as_ref().ok_or_else(|| {
        ApiError::bad_request(
            "Spotify is not configured; set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET",
        )
    })?;
    let redirect_uri = state.spotify_redirect_uri.as_deref().ok_or_else(|| {
        ApiError::bad_request("SPOTIFY_REDIRECT_URI is not configured on the server")
    })?;

    let oauth_state = Uuid::new_v4().to_string();

    sqlx::query(
        "DELETE FROM spotify_oauth_states WHERE created_at < now() - interval '15 minutes'",
    )
    .execute(&state.db)
    .await?;

    sqlx::query("INSERT INTO spotify_oauth_states (state, user_id) VALUES ($1, $2)")
        .bind(&oauth_state)
        .bind(admin.0.id)
        .execute(&state.db)
        .await?;

    let url = client
        .authorize_url(redirect_uri, &oauth_state)
        .map_err(|error| map_error(error, false))?;

    tracing::info!(
        admin_id = %admin.0.id,
        redirect_uri = %redirect_uri,
        "Spotify OAuth authorization started"
    );

    Ok(Json(AuthorizeResponse { url }))
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Redirect {
    match handle_callback(&state, query).await {
        Ok(()) => Redirect::to("/admin?spotify=connected"),
        Err(error) => {
            tracing::warn!(error = ?error, "Spotify OAuth callback failed");
            Redirect::to("/admin?spotify=error")
        }
    }
}

async fn handle_callback(state: &AppState, query: CallbackQuery) -> anyhow::Result<()> {
    let oauth_state = query.state.context("missing OAuth state")?;

    // Single use, short lived, and bound to the admin who started the flow.
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "DELETE FROM spotify_oauth_states \
         WHERE state = $1 AND created_at > now() - interval '15 minutes' \
         RETURNING user_id",
    )
    .bind(&oauth_state)
    .fetch_optional(&state.db)
    .await?;

    let user_id = user_id.context("invalid or expired OAuth state")?;

    if let Some(error) = query.error {
        bail!("Spotify returned an authorization error: {error}");
    }

    let code = query.code.context("missing authorization code")?;
    let client = state
        .spotify
        .as_ref()
        .context("Spotify is not configured")?;
    let redirect_uri = state
        .spotify_redirect_uri
        .as_deref()
        .context("SPOTIFY_REDIRECT_URI is not configured")?;

    let connection = spotify_auth::connect(client, &state.db, &code, redirect_uri, user_id).await?;

    tracing::info!(
        spotify_user_id = ?connection.spotify_user_id,
        display_name = ?connection.display_name,
        "Spotify OAuth connection established"
    );

    Ok(())
}

pub async fn disconnect(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<Json<DisconnectResponse>, ApiError> {
    spotify_auth::disconnect(&state.db)
        .await
        .map_err(|error| map_error(error, false))?;

    tracing::info!("Spotify account disconnected");

    Ok(Json(DisconnectResponse { connected: false }))
}

/// Maps Spotify failures to client-safe API errors and logs the detail.
pub fn map_error(error: SpotifyError, connected: bool) -> ApiError {
    match error {
        SpotifyError::NotFound => {
            ApiError::NotFound("playlist not found or not accessible".to_string())
        }
        SpotifyError::Forbidden => {
            tracing::warn!(
                connected,
                "Spotify denied access to the playlist; it may be restricted or owned by another account"
            );
            if connected {
                ApiError::Forbidden(
                    "Spotify denied access. The connected account must own or collaborate on this playlist."
                        .to_string(),
                )
            } else {
                ApiError::Forbidden(
                    "Spotify denied access. Connect your Spotify account in the Playlist panel and try again — development-mode apps require a user connection."
                        .to_string(),
                )
            }
        }
        SpotifyError::RateLimited => ApiError::service_unavailable(
            "Spotify is rate limiting requests; try again in a minute",
        ),
        SpotifyError::Unauthorized => {
            tracing::error!("Spotify rejected the configured app credentials");
            ApiError::internal(anyhow::anyhow!("Spotify app credentials were rejected"))
        }
        SpotifyError::Reconnect => ApiError::bad_request(
            "The Spotify connection has expired. Reconnect your Spotify account in the Playlist panel.",
        ),
        SpotifyError::Api { status, message } => {
            tracing::error!(status, message = %message, "Spotify API error");
            if status >= 500 {
                ApiError::service_unavailable(
                    "Spotify is temporarily unavailable; try again in a minute",
                )
            } else {
                ApiError::internal(anyhow::anyhow!("Spotify API error ({status})"))
            }
        }
        SpotifyError::Other(error) => {
            // Transport/DNS/TLS and response-parse failures are upstream
            // availability problems, not a bug in this application.
            tracing::error!(error = ?error, "could not reach or parse Spotify");
            ApiError::service_unavailable(
                "Could not reach Spotify. Check the connection and try again.",
            )
        }
    }
}
