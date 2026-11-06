use std::sync::Arc;
use std::time::Instant;

use sqlx::PgPool;
use tokio::sync::Semaphore;

use crate::auth::AuthService;
use crate::cache::{AuthUserCache, ResponseCache};
use crate::rate_limit::RateLimiter;
use crate::spotify::SpotifyClient;

/// Shared application state, cloned into every request handler.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth: AuthService,
    pub spotify: Option<SpotifyClient>,
    pub spotify_redirect_uri: Option<String>,
    pub rate_limiter: Arc<RateLimiter>,
    pub login_rate_limit: u32,
    pub register_rate_limit: u32,
    pub started_at: Instant,
    pub environment: String,
    /// Cached public aggregates with per-key single-flight.
    pub cache: Arc<ResponseCache>,
    /// Short-lived authenticated user cache; invalidated on admin changes.
    pub auth_cache: Arc<AuthUserCache>,
    /// Caps concurrent heavy analytics queries so they cannot starve votes.
    pub heavy_work: Arc<Semaphore>,
}
