use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::state::AppState;

pub mod admin;
pub mod aggregate;
pub mod auth;
pub mod chart;
pub mod dashboard;
pub mod history;
pub mod matchup;
pub mod playlist;
pub mod ranking;
pub mod settings;
pub mod spotify;
pub mod stories;
pub mod streak;
pub mod users;
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
        .route("/chart/artists", get(aggregate::artists))
        .route("/chart/albums", get(aggregate::albums))
        .route("/chart/stories", get(stories::public))
        .route("/vote", post(vote::cast))
        .route("/vote/undo", post(vote::undo))
        .route("/me/ranking", get(ranking::personal))
        .route("/me/summary", get(ranking::summary))
        .route("/me/duels", get(history::list))
        .route("/admin/reveal", post(chart::reveal))
        .route("/admin/reveal/schedule", post(chart::schedule_reveal))
        .route(
            "/admin/reveal/schedule/cancel",
            post(chart::cancel_scheduled_reveal),
        )
        .route("/admin/chart.csv", get(chart::export_csv))
        .route("/admin/stories", get(stories::admin))
        .route("/admin/stats", get(dashboard::stats))
        .route(
            "/admin/settings",
            get(settings::get_settings).patch(settings::update_settings),
        )
        .route("/admin/insights", get(dashboard::insights))
        .route("/admin/users", get(dashboard::users))
        .route("/admin/users.csv", get(dashboard::users_csv))
        .route("/admin/users/{id}", delete(users::delete_user))
        .route("/admin/users/{id}/clear-votes", post(users::clear_votes))
        .route("/admin/users/{id}/username", patch(users::rename_user))
        .route("/admin/users/{id}/password", post(users::reset_password))
        .route("/admin/users/{id}/disabled", post(users::set_disabled))
        .route("/admin/users/{id}/logout", post(users::force_logout))
        .route("/admin/ranking", get(dashboard::ranking))
        .route("/admin/reset/votes", post(dashboard::reset_votes))
        .route("/admin/reset/all", post(dashboard::full_reset))
        .route("/admin/playlist/remove", post(dashboard::remove_playlist))
        .route(
            "/admin/invites",
            get(admin::list_invites).post(admin::create_invite),
        )
        .route("/admin/invites/{id}", delete(admin::revoke_invite))
        .route("/admin/playlist", get(playlist::overview))
        .route("/admin/playlist/import", post(playlist::import))
        .route(
            "/admin/playlist/schedule",
            post(playlist::schedule_playlist),
        )
        .route("/admin/playlist/start", post(playlist::start_playlist_now))
        .route(
            "/admin/playlist/schedule/cancel",
            post(playlist::cancel_scheduled_playlist),
        )
        .route("/admin/voting/pause", post(playlist::set_voting_paused))
        .route("/admin/playlist/tracks", get(playlist::tracks))
        .route("/admin/metadata", get(playlist::metadata_status))
        .route(
            "/admin/metadata/backfill",
            post(playlist::backfill_metadata),
        )
        .route(
            "/admin/previews/backfill",
            post(playlist::backfill_previews),
        )
        .route("/admin/spotify/status", get(spotify::status))
        .route("/admin/spotify/authorize", get(spotify::authorize))
        .route("/admin/spotify/callback", get(spotify::callback))
        .route("/admin/spotify/disconnect", post(spotify::disconnect))
}
