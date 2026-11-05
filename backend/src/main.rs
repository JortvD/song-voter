mod api;
mod audit;
mod auth;
mod config;
mod db;
mod elo;
mod error;
mod models;
mod preview_backfill;
mod rate_limit;
mod routes;
mod spotify;
mod spotify_auth;
mod state;

use std::time::Duration;

use anyhow::Context;
use axum::http::{header, HeaderName, HeaderValue, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::{info, info_span, warn, Level};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::auth::AuthService;
use crate::config::Config;
use crate::spotify::SpotifyClient;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    init_tracing(&config.log_filter);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        environment = %config.environment,
        frontend_dir = %config.frontend_dir,
        "starting ULL Top 2000 backend"
    );

    if config.jwt_secret_was_generated {
        warn!(
            "JWT_SECRET is not set; using an ephemeral secret. Sessions reset on every restart."
        );
    } else if config.jwt_secret.len() < 32 {
        warn!("JWT_SECRET is shorter than 32 characters; use a longer random value.");
    }

    info!(
        cookie_secure = config.cookie_secure,
        ttl_hours = config.jwt_ttl_hours,
        "authentication configured"
    );

    if config.spotify_client_id.is_some() && config.spotify_client_secret.is_some() {
        if config.spotify_redirect_uri.is_none() {
            warn!(
                "SPOTIFY_REDIRECT_URI is not set; Spotify account connection is disabled. \
                 Development Mode apps need a connected account to read playlists."
            );
        } else {
            info!(
                redirect_uri = %config.spotify_redirect_uri.as_deref().unwrap_or_default(),
                "Spotify OAuth connection configured"
            );
        }
    }

    let spotify = match (
        &config.spotify_client_id,
        &config.spotify_client_secret,
    ) {
        (Some(client_id), Some(client_secret)) => {
            info!(
                api_base = %config.spotify_api_base,
                "Spotify playlist import configured"
            );
            Some(SpotifyClient::with_bases(
                client_id.clone(),
                client_secret.clone(),
                config.spotify_api_base.clone(),
                config.spotify_accounts_base.clone(),
            )
            .with_authorize_url(config.spotify_authorize_url.clone())
            .with_embed_base(config.spotify_embed_base.clone()))
        }
        (None, None) => {
            warn!("Spotify import disabled (SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET not set)");
            None
        }
        _ => {
            warn!(
                "Spotify import disabled: set both SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET"
            );
            None
        }
    };

    let db = connect_database(&config.database_url).await?;
    info!("database connection pool ready, migrations applied");

    bootstrap_admin(&db, &config).await?;

    let state = AppState {
        db,
        auth: AuthService::new(
            &config.jwt_secret,
            config.jwt_ttl_hours,
            config.cookie_secure,
        ),
        spotify,
        spotify_redirect_uri: config.spotify_redirect_uri.clone(),
        rate_limiter: std::sync::Arc::new(crate::rate_limit::RateLimiter::new()),
        login_rate_limit: config.login_rate_limit,
        register_rate_limit: config.register_rate_limit,
        started_at: std::time::Instant::now(),
        environment: config.environment.clone(),
    };

    let app = build_router(state, &config.frontend_dir);
    let bind_address = config.bind_address();
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .with_context(|| format!("failed to bind {bind_address}"))?;

    info!(address = %bind_address, "HTTP server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server error")?;

    info!("HTTP server stopped, goodbye");

    Ok(())
}

fn init_tracing(default_level: &str) {
    // An empty RUST_LOG (common in compose env files) must not disable logging.
    let filter = std::env::var("RUST_LOG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .and_then(|value| EnvFilter::try_new(value).ok())
        .unwrap_or_else(|| EnvFilter::new(default_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_ansi(false)
        .init();
}

async fn connect_database(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    Ok(pool)
}

/// Creates (or promotes) the admin account configured via environment variables,
/// so the very first login never depends on an invite code.
async fn bootstrap_admin(pool: &PgPool, config: &Config) -> anyhow::Result<()> {
    let (Some(username), Some(password)) = (&config.admin_username, &config.admin_password) else {
        info!("admin bootstrap disabled (ADMIN_USERNAME or ADMIN_PASSWORD not set)");
        return Ok(());
    };
    let username = username.trim();

    let existing = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE lower(username) = lower($1)",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    if let Some(user_id) = existing {
        sqlx::query("UPDATE users SET is_admin = TRUE WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;
        info!(%user_id, username, "bootstrap admin account already exists");
        return Ok(());
    }

    let password_hash = auth::hash_password(password).await?;
    let admin = sqlx::query_as::<_, models::User>(
        "INSERT INTO users (username, password_hash, is_admin) VALUES ($1, $2, TRUE) \
         RETURNING id, username, password_hash, is_admin, created_at",
    )
    .bind(username)
    .bind(&password_hash)
    .fetch_one(pool)
    .await?;

    info!(user_id = %admin.id, username = %admin.username, "bootstrap admin account created");

    Ok(())
}

fn build_router(state: AppState, frontend_dir: &str) -> Router {
    const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; \
        img-src 'self' https://*.scdn.co data:; \
        media-src https://p.scdn.co; \
        style-src 'self' 'unsafe-inline'; \
        script-src 'self'; \
        connect-src 'self'; \
        font-src 'self'; \
        frame-ancestors 'none'; \
        base-uri 'self'; \
        form-action 'self'";

    let index_file = format!("{frontend_dir}/index.html");
    let static_files = ServeDir::new(frontend_dir).fallback(ServeFile::new(index_file));

    let api = Router::new()
        .route("/health", get(routes::health))
        .merge(api::router())
        .fallback(routes::api_not_found)
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));

    Router::new()
        .route("/healthz", get(routes::health))
        .route("/readyz", get(routes::ready))
        .nest("/api", api)
        .fallback_service(static_files)
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(
                            |request: &Request<_>| {
                                let request_id = request
                                    .headers()
                                    .get("x-request-id")
                                    .and_then(|value| value.to_str().ok())
                                    .unwrap_or("-");
                                info_span!(
                                    "http_request",
                                    method = %request.method(),
                                    uri = %request.uri(),
                                    request_id = %request_id,
                                )
                            },
                        )
                        .on_response(DefaultOnResponse::new().level(Level::INFO))
                        .on_failure(DefaultOnFailure::new().level(Level::ERROR)),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(60),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CONTENT_SECURITY_POLICY),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .with_state(state)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    warn!("shutdown signal received, draining connections");
}
