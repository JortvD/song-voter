# Backend — AGENTS.md

Supplement to `../AGENTS.md`. Read the root file first for hard boundaries,
commands and verification expectations. This file covers the Rust service in
`backend/`.

## Stack and layout

Rust 2021, axum 0.8, sqlx 0.8 with **runtime** queries, tokio, tracing, reqwest
(rustls), argon2, jsonwebtoken. PostgreSQL 16. Migrations run automatically at
startup.

| Path | Responsibility |
|---|---|
| `src/main.rs` | tracing, config, DB pool, admin bootstrap, router, middleware, shutdown |
| `src/config.rs` | all environment parsing and production secret checks |
| `src/state.rs` | `AppState`: pool, auth, Spotify, caches, rate limiter, semaphore |
| `src/routes.rs` | `/healthz`, `/readyz`, JSON API 404 |
| `src/error.rs` | `ApiError`, `{"error": …}` envelope, input validation helpers |
| `src/auth.rs` | Argon2, JWT, session cookie, `AuthUser` / `AdminUser` extractors |
| `src/audit.rs` | `auth_events` audit trail (failures never fail the request) |
| `src/db.rs` | shared playlist/progress lookups and voting-block reasons |
| `src/models.rs` | `User`, `Invite`, `Playlist`, `Track`, `MatchupTrack` |
| `src/elo.rs` | Elo maths and K-factor decay |
| `src/ratings.rs` | vote-ledger replay and standings rebuilds |
| `src/cache.rs` | in-process `ResponseCache` and `AuthUserCache` |
| `src/rate_limit.rs` | fixed-window per-IP limiter |
| `src/scheduler.rs` | 15s loop for scheduled playlist starts and reveals |
| `src/metadata.rs` | Spotify metadata/genre parsing and backfill helpers |
| `src/metadata_backfill.rs` | background metadata/genre run lifecycle |
| `src/preview_backfill.rs` | background preview-URL scraping from embed pages |
| `src/spotify.rs` | Spotify API client, playlist pagination, ID parsing, OAuth URL |
| `src/spotify_auth.rs` | stored tokens, refresh, connection status |
| `src/api/` | HTTP handlers, split by feature (see `api/mod.rs`) |

`api/` layout: `auth`, `admin`, `playlist`, `spotify`, `matchup`, `vote`,
`history`, `streak`, `ranking`, `chart`, `aggregate`, `stories`, `track_detail`,
`dashboard`, `admin_stats`, `admin_tracks`, `admin_voters`, `users`, `settings`.

## Commands

```bash
cd backend
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets
cargo test
```

These do not need a database. If you need to run the service, see the root file
for `DATABASE_URL`, `JWT_SECRET`, `ADMIN_*` and `COOKIE_SECURE=false`.

## Handler conventions

Use the existing shape; keep handlers thin and put shared logic in `db.rs`,
`ratings.rs`, `models.rs` or a feature module.

```rust
pub async fn handler(
    State(state): State<AppState>,
    user: AuthUser,              // or admin: AdminUser
    Query(query): Query<SomeQuery>,
) -> Result<Json<SomeResponse>, ApiError> {
    // validate input early
    // one transaction for multi-row writes
    // return a response struct, never raw database rows with secrets
}
```

- Authenticated routes take `AuthUser`; admin routes take `AdminUser`. Never
  hand-roll a session check.
- Deserialize request bodies with typed `serde` structs and `Json<T>`.
  `json_api_errors` normalises axum rejections to the JSON envelope.
- Return `ApiError::bad_request/unauthorized/forbidden/not_found/conflict/
  too_many_requests/service_unavailable`, or `ApiError::internal(err)` for real
  failures. `Internal` logs the detail and returns a generic message.
- Use `validate_text_input` on user-supplied search/filter text before SQL.
  Use `is_unique_violation` rather than matching error strings by hand.
- Serialize only fields the client needs. Public/pre-reveal responses must not
  include ratings, wins or comparison counts.
- Responses use snake_case (serde default). The frontend types mirror these
  exact names; do not rename fields without updating `frontend/src/lib/types.ts`.

## Error contract

Every API error is `{"error": "<safe message>"}` with the matching status.
`main.rs` also rewrites plain-text extractor rejections (400/405/413/415/422)
into this envelope. Unknown `/api/*` paths return JSON 404, never the SPA.

`sqlx::Error::PoolTimedOut` maps to 503. Do not leak SQL, tokens, Spotify bodies
or internal paths in the message.

## Authentication and authorization

- Registration is invite-only; invites are single-use 10-char uppercase hex.
- Passwords use Argon2. Sessions are JWTs in the `pv_session` HttpOnly,
  SameSite=Lax cookie (`COOKIE_SECURE` in production).
- `AuthUser` loads the user row (5s in-process cache) and verifies
  `session_version` so password resets/forced logouts revoke sessions.
- `AdminUser` is `AuthUser` plus `is_admin`.
- Any mutation that disables a user, resets a password, renames a user, changes
  admin status or forces logout **must** invalidate `state.auth_cache` for that
  user. Full resets should clear it entirely.
- Log auth events through `audit::record` (login, failed login, register,
  logout, invite creation). Audit failures are logged, not propagated.
- Login and registration rate limits use the client IP derived from
  `X-Forwarded-For`; the nginx vhost must keep setting it.

## Database and migrations

- Migrations live in `backend/migrations/` as `NNNN_snake_case.sql` (use the
  next unused number). They are **append-only**. Editing an applied migration
  changes its checksum and breaks `sqlx::migrate!` at startup.
- There are no down migrations. Make forward migrations safe for existing data:
  add nullable columns or defaults, backfill, then tighten constraints in a
  later migration if needed.
- `sqlx::migrate!("./migrations")` runs on every startup; keep individual
  statements fast enough for a deploy.
- Use runtime queries: `sqlx::query`, `sqlx::query_as::<_, T>`,
  `sqlx::query_scalar`. List columns explicitly instead of `SELECT *`.
  Define `#[derive(sqlx::FromRow)]` structs or tuple rows for internal shapes.
- Use `bind(...)` parameters — never interpolate user input into SQL.
- Multi-row state changes (votes, undos, deletes, rebuilds, imports) belong in
  one `state.db.begin()` transaction. Lock both track rows in UUID order
  (`track_low < track_high`) to avoid deadlocks and races.
- The pool is 16 connections with a 5s acquire timeout and a 15s
  `statement_timeout` per connection. HTTP requests time out at 180s.
- PostgreSQL unique violations are code `23505`; use `is_unique_violation`.
- Use `gen_random_uuid()` for UUID primary keys and `TIMESTAMPTZ DEFAULT now()`.

### Schema rules that matter

- At most one playlist is `is_active`; the active playlist is the season.
- `seen_pairs` normalises `track_low < track_high` and records served pairs per
  user/playlist. A pair is never served twice to the same voter, even after
  undo.
- `votes` stores `rating_*_before`, `delta_*`, `k_factor`, comparison counts and
  a monotonic `seq`. Undo subtracts the stored deltas; it does not restore
  snapshots. Rebuilds replay votes in `seq` order.
- Tracks removed from Spotify are not deleted on re-import, preserving history.
- Destructive admin actions require exact confirmation payloads:
  `{"confirm":"RESET EVERYTHING"}` (full reset), `{"confirm":"RESET"}` (votes),
  `{"confirm":"REMOVE"}` (track/playlist), `{"confirm":"CLEAR"}` (voter picks),
  and the target username for user deletion.
- The last active admin cannot be deleted or disabled.

## Scoring and standings

- Both tracks start at `1000` (`elo::INITIAL_RATING`).
- `K = max(16, 40 - min(comparisons_low, comparisons_high) * 0.5)`.
- `expected = 1 / (1 + 10^((rating_high - rating_low) / 400))`.
- Deltas are zero-sum: `delta_high = -delta_low`.
- The vote response never exposes ratings or deltas.
- **All** standings rebuilds (voter clear/delete, track removal, reset) must use
  `ratings::replay_votes`, which applies the same Elo maths in `seq` order. Do
  not duplicate scoring formulas in handlers or SQL.
- The public standings order is score, then wins, then title. The admin CSV
  uses the same official order.

## Duel selection and voting

- `GET /api/matchup` picks from the 100 least-compared tracks (ties: least
  recently compared), prefers the closest score, excludes pairs already in
  `seen_pairs`, and returns `409` when no unseen pair remains.
- Rank weighting is configurable in `api/settings.rs` and clamped server-side.
  Constants and defaults live there; do not scatter magic numbers.
- Presentation side is random; `served_flip` records how it was shown.
- `POST /api/vote` accepts only a served pair in the active playlist, once.
  Voting must be blocked when the season has not started or is paused
  (`db::voting_block_reason*`).
- `POST /api/vote/undo` reverses only the voter's latest pick in the active
  playlist and leaves the pair in `seen_pairs`.
- The daily streak is app-wide, uses the browser's local day (`tz` query
  parameter) and resets after a full missed day. The goal is a setting.

## Reveal rules

- `reveal_state` is `hidden` by default. Public chart, aggregate and story
  endpoints must return no standings data while hidden; `GET /api/chart/status`
  may reveal only the sealed state, cover, counts and facets.
- Admin standings/stories remain available while sealed.
- A manual reveal or seal clears a pending scheduled reveal.
- Revealing does not stop voting; that is a separate `voting_paused` flag.
- `pause_voting_on_reveal` optionally pauses when a scheduled reveal fires.

## Caching

- `ResponseCache` caches expensive non-personal JSON with single-flight and
  short TTLs (10s for chart/aggregates, 30s for stories). Explicit
  `state.cache.clear()` is used for structural/admin mutations: imports,
  settings, reveal, track/user removal, resets and scheduled transitions. Votes
  and undos rely on the short TTL; do not lengthen those TTLs without a
  cache-invalidation plan.
- `AuthUserCache` is a 5-second user-row cache. Invalidate the affected user
  after admin user mutations; clear it after global resets.
- Cached values must not contain voter-specific data or secrets.

## Background work

- `scheduler::spawn` ticks every 15s. It starts due playlists (retrying failed
  Spotify imports every 60s) and opens due reveals. Keep tick work short and
  idempotent; log errors instead of panicking.
- Metadata backfill runs as a background task and records progress in
  `metadata_backfill_runs`. On startup `fail_orphaned_runs` marks leftover
  `running` rows failed so the admin retry button works.
- Preview backfill fetches missing 30s preview URLs from public Spotify embed
  pages with bounded concurrency.
- Heavy analytics handlers acquire `state.heavy_work` (3 permits) so they cannot
  starve voting. Preserve that for new expensive endpoints.
- Never block a request on a long backfill; start it, return progress, and let
  the admin poll.

## Spotify integration

- Import accepts a playlist URL, `spotify:playlist:` URI or bare 22-char ID
  (`spotify::parse_playlist_id`). Keep parsing covered by unit tests.
- Import pages of 50 with retry/backoff for 429/5xx. Skip local files, podcast
  episodes, unavailable tracks and duplicates, and report counts.
- Re-importing the same playlist upserts and preserves scores; importing a
  different playlist deactivates the previous one.
- OAuth is admin-only; tokens are stored server-side and refreshed
  automatically. Never log authorization codes, access tokens or refresh
  tokens.
- Spotify base URLs are injectable (`with_bases`) for `wiremock` tests. Keep new
  HTTP calls testable the same way.

## Testing

- Unit tests live at the bottom of each module in `#[cfg(test)] mod tests`.
  Pure logic is preferred: Elo (`elo.rs`), replay (`ratings.rs`), auth
  validation (`auth.rs`), metadata parsing (`metadata.rs`), Spotify parsing and
  pagination (`spotify.rs`).
- `wiremock` is a dev-dependency for Spotify HTTP tests; use `MockServer`
  instead of hitting the real API.
- There is no database test harness. If you add one, keep `cargo test`
  runnable without external services by default (opt-in env var), otherwise it
  breaks the standard verification path.
- Add a regression test whenever you fix scoring, pairing, streak, parsing or
  authorization logic.

## Security, logging and performance

- Never log secrets, cookies, JWTs, OAuth codes, tokens, password hashes or
  full user rows.
- Use structured `tracing` fields (`%user_id`, `%playlist_id`, `?error`).
  `RUST_LOG` overrides `LOG_LEVEL`; an empty `RUST_LOG` must keep working.
- Keep the documented security headers in `main.rs` (CSP, frame denial,
  nosniff, referrer and permissions policies). If you add a new external asset
  origin (images, fonts, frames, media), update the CSP instead of removing it.
- Respect the `Cache-Control` split: immutable for hashed `/assets` and
  `/fonts`, `no-cache` for the SPA shell, `no-store` for `/api`.
- Do not add compile-time SQL macros, new global state, or unbounded tasks
  without discussing the operational impact.

## Adding an endpoint — checklist

1. Add or extend a module under `src/api/`, with typed request/response structs.
2. Choose `AuthUser`, `AdminUser` or public; validate input; use `ApiError`.
3. Put reusable queries in `db.rs` / the feature module; use a transaction for
   multi-row writes; lock tracks in UUID order.
4. Register the route in `src/api/mod.rs` under the correct prefix.
5. Invalidate `ResponseCache` / `AuthUserCache` if the mutation changes cached
   or user-visible data.
6. Add a unit test for pure logic; update `README.md`'s API table if the
   endpoint is user- or admin-facing.
7. Update `frontend/src/lib/types.ts` and `frontend/src/lib/api.ts` if the UI
   consumes it.
8. Run `cargo fmt --check`, `cargo check --all-targets`,
   `cargo clippy --all-targets` and `cargo test`.
