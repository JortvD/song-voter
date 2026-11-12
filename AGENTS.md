# AGENTS.md

Instructions for coding agents working in this repository. Read this file first,
then the folder-level file that matches the code you touch:

- `backend/AGENTS.md` — Rust/axum/sqlx backend
- `frontend/AGENTS.md` — React/TypeScript/Vite frontend

`README.md` is the authoritative product and operations manual. It is long;
read the sections relevant to your change instead of skimming the whole file.

## What this project is

**Playlist Voter** (repo/container name `ull-top2000`) is a collaborative music
ranking game. Voters are shown two tracks (a **duel**) and pick a winner.
Picks update a shared Elo score. Voters can always see **Your Ranking** (built
from their own picks), while the official **The Standings** stay hidden until an
admin opens **The Reveal**. A mission-control admin area manages the Spotify
playlist, voters, invites, settings and the reveal.

The project is feature-complete and running in production. Prefer small,
consistent changes over rewrites.

## Stack at a glance

| Layer | Technology |
|---|---|
| Backend | Rust 2021, axum 0.8, sqlx 0.8 (runtime queries), tokio, tracing |
| Frontend | React 19, TypeScript 5.9 (strict), Vite 7, Tailwind CSS 4 |
| Database | PostgreSQL 16; migrations run automatically on backend startup |
| Packaging | Multi-stage Dockerfile; backend serves the built SPA and `/api` |
| Edge | Host nginx terminates TLS and proxies to `127.0.0.1:8765` |

There is no CI configuration and no frontend test/lint script. Verification is
local: `cargo` checks plus `npm run build` (which runs `tsc --noEmit`).

## Hard boundaries

These are non-negotiable:

1. **Never edit, create, print or commit `.env` or any real secret.** Use
   `.env.example` to document new variables. Do not put credentials, JWTs,
   Spotify tokens or password hashes in code, tests, logs or commit messages.
   The existing invite-creation flow is the only place that logs an invite code.
2. **Never edit a migration that may already have been applied.** Migrations
   are append-only. Add the next numbered file in `backend/migrations/`.
3. **Never deploy or touch production.** That includes SSH, production
   `docker compose`, production DB access, `scripts/backup.sh` /
   `scripts/restore.sh` against production, DNS, nginx or certbot. Local Docker
   Compose is fine.
4. **Never commit build artifacts or local state:** `backend/target/`,
   `frontend/node_modules/`, `frontend/dist/`, `backups/`, `*.log`, `.env`.

Also respect these project invariants unless the task explicitly changes them:

- Public, pre-reveal responses must not leak scores, ratings, wins or comparison
  counts. The duel response exposes a deliberately public track shape.
- A pick is valid only for a pair that was served to that voter in the active
  playlist, and only once. The pair stays in `seen_pairs` after an undo.
- Elo deltas are zero-sum; undo subtracts stored deltas instead of restoring
  snapshots; rebuilds replay the vote ledger. Never reimplement scoring ad hoc.
- API failures are JSON `{"error": "..."}`. Never return internal error details,
  SQL text or Spotify error bodies to clients.
- Admin-only routes must use the `AdminUser` extractor. Destructive endpoints
  keep their exact typed-confirmation strings.

## Repository map

```
.
├── AGENTS.md                 # this file
├── README.md                 # product, API, deployment and operations manual
├── compose.yaml              # db + backend production-like stack
├── Dockerfile                # node build → cargo build → slim runtime
├── .env.example              # documented configuration (never copy secrets in)
├── backend/
│   ├── Cargo.toml
│   ├── migrations/           # append-only sqlx migrations, 0001…0016
│   └── src/                  # see backend/AGENTS.md
├── frontend/
│   ├── package.json
│   ├── vite.config.ts        # dev server proxies /api, /healthz, /readyz
│   └── src/                  # see frontend/AGENTS.md
├── scripts/                  # backup.sh, restore.sh (production ops)
└── deploy/nginx/             # certbot-ready vhost
```

## Commands

Run from the repository root unless noted.

### Backend (host toolchain)

```bash
cd backend
cargo fmt --check          # formatting gate
cargo check --all-targets  # compile checks, including tests
cargo clippy --all-targets # lint; fix new warnings you introduce
cargo test                 # unit tests
```

No database is needed for these: the backend uses runtime `sqlx::query*`
strings, not compile-time query macros. Only `sqlx::migrate!` is embedded at
compile time. Do **not** introduce `sqlx::query!`/`query_as!` macros unless the
project is also switched to sqlx offline metadata (`.sqlx/` + `SQLX_OFFLINE`).

### Frontend

```bash
cd frontend
npm ci
npm run build   # tsc --noEmit && vite build — required gate
npm run dev     # http://localhost:5173, proxies API calls
```

If Node/npm is unavailable in the agent environment, use the Docker build stage
instead:

```bash
docker build --target frontend -t ull-top2000-frontend-check .
```

### Full stack (local only)

Requires an already configured `.env` with `POSTGRES_PASSWORD`, `JWT_SECRET`,
`ADMIN_USERNAME` and `ADMIN_PASSWORD`. Ask the user to set it up; do not create
or edit `.env`, and never commit it.

```bash
docker compose up -d --build
docker compose logs -f backend
```

Smoke checks: `curl -fsS http://localhost:8765/healthz` (liveness) and
`curl -fsS http://localhost:8765/readyz` (PostgreSQL readiness).

### Backend outside Compose

`DATABASE_URL` must point at a reachable PostgreSQL. The compose `db` service
does not publish a port; either run the backend in compose, or temporarily
publish `127.0.0.1:5432:5432` for local work (do not commit that change).

```bash
cd backend
export DATABASE_URL=postgres://ulltop2000:<password>@localhost:5432/ulltop2000
export JWT_SECRET=$(openssl rand -hex 32)
export ADMIN_USERNAME=admin ADMIN_PASSWORD=local-dev-only
export APP_ENV=development COOKIE_SECURE=false
cargo run
```

Migrations run automatically on startup. For the SPA shell, build the frontend
first (`FRONTEND_DIR` defaults to `./frontend/dist`) or use `npm run dev`.

## How a request flows

1. axum router (`backend/src/main.rs`) applies compression, request IDs,
   tracing, timeout, security headers and SPA/static file fallbacks.
2. `/api/*` is nested under `backend/src/api/mod.rs`; every response is
   `Cache-Control: no-store`.
3. Handlers receive `State<AppState>`, an auth extractor (`AuthUser` or
   `AdminUser`) and extractors such as `Json`/`Query`.
4. Errors become `ApiError` (`backend/src/error.rs`) → JSON envelope.
5. The frontend calls `/api/*` through `frontend/src/lib/api.ts`; the Vite dev
   server proxies to the backend, and production serves the built SPA from the
   same origin.

## Product language vs. API language

Keep UI copy, docs and commit messages in English. Use the product vocabulary in
user-facing text and the legacy/internal names in API paths and database tables.

| Product / UI | API & internal |
|---|---|
| duel | `matchup`, `duels`, `track_low`/`track_high` |
| pick | `vote` |
| Your Ranking | `/api/me/ranking`, `personal` ranking |
| The Standings | `chart`, `standings` |
| The Reveal | `reveal_state` (`hidden`/`revealed`) |
| The Stats | `/stats`, stories, aggregates |
| season / playlist | `playlists`, active playlist |

Do not rename existing API paths to match the product vocabulary without an
explicit compatibility plan. Frontend routes already use product names
(`/duel`, `/your-ranking`, `/standings`) with redirects from the old paths.

## Verification and definition of done

Before calling a change complete:

1. For Rust changes, run `cargo fmt --check`, `cargo check --all-targets`,
   `cargo clippy --all-targets` and `cargo test` in `backend/`.
2. For TypeScript/TSX/CSS changes, run `npm run build` in `frontend/`. The strict
   config fails on unused locals/parameters and type-only imports must use
   `import type`.
3. Add or update unit tests for pure logic changes (Elo, replay, streak,
   metadata parsing, auth validation).
4. Exercise the affected flow locally when the change touches requests,
   database behaviour, voting, reveal or admin actions. State what you could
   not run.
5. Keep migrations and deployed-API compatibility in mind. A change that needs
   a data backfill should be safe to run against existing data.
6. Summarize the change, verification performed, and any remaining risk.

Do not claim a check passed if it was not run. If Docker or a database is
unavailable, say so and list the exact command the user should run.

## Commits

Match the existing history: Conventional Commits with a short, imperative
subject, optionally scoped.

```
feat(admin): add numbered pagination and URL-backed tabs
fix(duel): keep preview state across undo
docs: document metadata backfill
chore: remove old docs
```

Keep commits focused. Do not mix formatting-only churn with behaviour changes.
Do not include generated files, secrets or unrelated refactors.

## Where to look next

- Product behaviour, API table, scheduled seasons, security and backups:
  `README.md`.
- Backend patterns, SQL rules and scoring internals: `backend/AGENTS.md`.
- UI patterns, Tailwind tokens and API client rules: `frontend/AGENTS.md`.
- Visual style reference: the `/design` route (`frontend/src/pages/DesignPage.tsx`).
