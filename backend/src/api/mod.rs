use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub mod admin;
pub mod auth;
pub mod balanced;
pub mod chart;
pub mod dashboard;
pub mod matchup;
pub mod playlist;
pub mod ranking;
pub mod spotify;
pub mod vote;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        .route("/matchup", get(matchup::next))
        .route("/chart/status", get(chart::status))
        .route("/chart", get(chart::public))
        .route("/chart/balanced", get(balanced::public))
        .route("/vote", post(vote::cast))
        .route("/vote/undo", post(vote::undo))
        .route("/me/ranking", get(ranking::personal))
        .route("/admin/reveal", post(chart::reveal))
        .route("/admin/chart.csv", get(chart::export_csv))
        .route("/admin/balanced", get(balanced::admin_view))
        .route(
            "/admin/balanced/settings",
            get(balanced::settings).put(balanced::update_settings),
        )
        .route("/admin/stats", get(dashboard::stats))
        .route("/admin/users", get(dashboard::users))
        .route("/admin/ranking", get(dashboard::ranking))
        .route("/admin/reset/votes", post(dashboard::reset_votes))
        .route("/admin/playlist/remove", post(dashboard::remove_playlist))
        .route(
            "/admin/invites",
            get(admin::list_invites).post(admin::create_invite),
        )
        .route("/admin/playlist", get(playlist::overview))
        .route("/admin/playlist/import", post(playlist::import))
        .route("/admin/playlist/tracks", get(playlist::tracks))
        .route("/admin/previews/backfill", post(playlist::backfill_previews))
        .route("/admin/spotify/status", get(spotify::status))
        .route("/admin/spotify/authorize", get(spotify::authorize))
        .route("/admin/spotify/callback", get(spotify::callback))
        .route("/admin/spotify/disconnect", post(spotify::disconnect))
}
