use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

use crate::spotify::{SpotifyClient, SpotifyError};

#[derive(Debug, sqlx::FromRow)]
struct StoredCredentials {
    access_token: String,
    refresh_token: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ConnectionInfo {
    pub spotify_user_id: Option<String>,
    pub display_name: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Returns a usable user access token, refreshing it when needed.
///
/// `None` means no Spotify account has been connected yet.
pub async fn user_access_token(
    client: &SpotifyClient,
    pool: &PgPool,
) -> Result<Option<String>, SpotifyError> {
    let stored = sqlx::query_as::<_, StoredCredentials>(
        "SELECT access_token, refresh_token, expires_at \
         FROM spotify_credentials WHERE id = TRUE",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| SpotifyError::Other(error.into()))?;

    let Some(stored) = stored else {
        return Ok(None);
    };

    if stored.expires_at > Utc::now() + Duration::seconds(60) {
        return Ok(Some(stored.access_token));
    }

    tracing::info!("Spotify user access token expired, refreshing");
    let tokens = client.refresh_user_token(&stored.refresh_token).await?;
    let refresh_token = tokens.refresh_token.unwrap_or(stored.refresh_token);
    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in.max(60));

    sqlx::query(
        "UPDATE spotify_credentials \
         SET access_token = $1, refresh_token = $2, expires_at = $3, updated_at = now() \
         WHERE id = TRUE",
    )
    .bind(&tokens.access_token)
    .bind(&refresh_token)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|error| SpotifyError::Other(error.into()))?;

    tracing::info!(expires_at = %expires_at, "Spotify user access token refreshed");

    Ok(Some(tokens.access_token))
}

pub async fn connect(
    client: &SpotifyClient,
    pool: &PgPool,
    code: &str,
    redirect_uri: &str,
    user_id: uuid::Uuid,
) -> Result<ConnectionInfo, SpotifyError> {
    let tokens = client.exchange_code(code, redirect_uri).await?;
    let refresh_token = tokens
        .refresh_token
        .clone()
        .ok_or_else(|| SpotifyError::Other(anyhow!("Spotify did not return a refresh token")))?;
    let spotify_user = client.current_user(&tokens.access_token).await?;
    let expires_at = Utc::now() + Duration::seconds(tokens.expires_in.max(60));

    let connection = sqlx::query_as::<_, ConnectionInfo>(
        "INSERT INTO spotify_credentials \
            (id, spotify_user_id, display_name, access_token, refresh_token, scope, \
             expires_at, connected_by, connected_at, updated_at) \
         VALUES (TRUE, $1, $2, $3, $4, $5, $6, $7, now(), now()) \
         ON CONFLICT (id) DO UPDATE SET \
            spotify_user_id = EXCLUDED.spotify_user_id, \
            display_name = EXCLUDED.display_name, \
            access_token = EXCLUDED.access_token, \
            refresh_token = EXCLUDED.refresh_token, \
            scope = EXCLUDED.scope, \
            expires_at = EXCLUDED.expires_at, \
            connected_by = EXCLUDED.connected_by, \
            connected_at = now(), \
            updated_at = now() \
         RETURNING spotify_user_id, display_name, connected_at, expires_at",
    )
    .bind(&spotify_user.id)
    .bind(spotify_user.display_name.as_deref())
    .bind(&tokens.access_token)
    .bind(&refresh_token)
    .bind(tokens.scope.as_deref())
    .bind(expires_at)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|error| SpotifyError::Other(error.into()))?;

    tracing::info!(
        spotify_user_id = %spotify_user.id,
        display_name = ?spotify_user.display_name,
        "Spotify account connected"
    );

    Ok(connection)
}

pub async fn status(pool: &PgPool) -> Result<Option<ConnectionInfo>, SpotifyError> {
    sqlx::query_as::<_, ConnectionInfo>(
        "SELECT spotify_user_id, display_name, connected_at, expires_at \
         FROM spotify_credentials WHERE id = TRUE",
    )
    .fetch_optional(pool)
    .await
    .map_err(|error| SpotifyError::Other(error.into()))
}

pub async fn disconnect(pool: &PgPool) -> Result<(), SpotifyError> {
    sqlx::query("DELETE FROM spotify_credentials WHERE id = TRUE")
        .execute(pool)
        .await
        .map_err(|error| SpotifyError::Other(error.into()))?;

    Ok(())
}
