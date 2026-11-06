# Release test report — Playlist Voter (ULL Top 2000)

**Date:** 2026-11-06  
**Frozen code:** `119c2ea` (“feat: data drawer in lists”) — the committed inline track-detail feature, duel-height/CSS changes, ranking/auth updates.  
**Tested artifact:** isolated Docker Compose stack rebuilt from that frozen tree; seeded with 150 tracks, 270 votes, 291 duels, 11 users, sealed and revealed states.  
**Result:** **42 findings fixed and verified on a rebuilt stack** (all 8 high blockers + 34 selected mediums). **15 medium and 82 low findings remain TBD** (see “Still TBD”).

## How to use this report

- IDs are stable. Reply with the IDs you want implemented, e.g. `SEC-001, SEC-100, BUG-150, BUG-152`.
- **High** — fix before release.
- **Medium** — fix before release if time allows, or explicitly accept the risk.
- **Low** — polish, hardening, and follow-up.
- “Verified” means reproduced on the frozen build; “code review” means inspected but not reproduced; “mock” means Spotify was exercised against the deterministic mock, not real Spotify.
- Aliases in brackets show duplicate reports that were merged into the canonical ID.
- Machine-readable index (139 high/medium/low rows): `docs/release-test-findings-2026-11-06.csv`.

## Release blocker status (fixed)

| ID | Status | Fix and verification |
|---|---|---|
| SEC-001 | ✅ **Fixed** | Production startup now requires an `ADMIN_PASSWORD` of at least 12 characters and rejects the admin username. Verified: `APP_ENV=production` + `ADMIN_PASSWORD=a` exits with “must be at least 12 characters”; a long password starts and logs in with 200. |
| SEC-100 | ✅ **Fixed** | Login usernames are limited to 1–64 control-free characters and passwords to 1–128 before Argon2/audit/log work. Verified: 1.5 MB, newline and NUL usernames return 401, add no `auth_events` rows, and normal login still returns 200. |
| BUG-150 | ✅ **Fixed** | Undo now deletes the vote and replays the remaining playlist ledger via `ratings::rebuild_playlist_ratings`, so it matches admin clear/delete. Verified in a 3-voter/2-track case: undo ratings exactly match an independent replay (997.986190466 / 1002.013809534) and the audit row before-values chain again. |
| A11Y-001 | ✅ **Fixed** | Vote/undo focus the next pick button, exhausted state focuses its heading, Load more focuses the status line, and auth errors focus the invalid field. Verified with keyboard-only Playwright runs. |
| A11Y-002 | ✅ **Fixed** | Loser text is no longer opacity-dimmed; only cover art is de-emphasised. Verified: axe reports **0** contrast violations on `/your-duels` (was 94). Remaining `/duel` contrast is the separate medium A11Y-006 (listen time/meta). |
| A11Y-003 | ✅ **Fixed** | Added `scroll-padding-block`/focus `scroll-margin-block` and extra short-viewport duel padding. Verified on 390×844 `/your-duels`: 90 preview buttons, **0** covered by the fixed nav; landscape controls clear the nav. |
| PERF-001 | ✅ **Fixed** | `lock_track` uses `FOR NO KEY UPDATE`, which is compatible with the `seen_pairs` FK `KEY SHARE` lock. Verified under 30-voter mixed load: 2,209 matchups + 2,179 votes, **0 errors / 0 deadlocks** in server logs. |
| PERF-003 | ✅ **Fixed** | `#main` reserves the viewport height, the playlist badge reserves its loaded height, and `/stats` waits for stories before mounting the explore section. Verified CLS: home 0.096, duel 0.099, standings 0.020, stats 0.020, ranking 0.020, admin 0.080 (was 0.65–1.60). |

## Second batch status (fixed)

| ID | Status | Fix and verification |
|---|---|---|
| SEC-102 | ✅ **Fixed** | Logout now bumps `session_version` and invalidates the auth cache. Verified: replaying the pre-logout cookie returns 401; a fresh login returns 200. |
| BUG-101 | ✅ **Fixed** | Control characters are rejected before search/filter SQL; NUL timezones fall back to UTC. Verified: NUL `q`/`genre`/admin search → 400 JSON; NUL login → 401; NUL `tz` still returns 200. |
| BUG-152 | ✅ **Fixed** | Undo reads `served_flip` and restores the original A/B orientation. Verified with a forced flipped pair: undo returned high/low exactly. |
| BUG-200 | ✅ **Fixed** | `/`, `/standings`, `/stats`, `/duel` now render a “season unavailable” card with Try again when `/api/chart/status` fails. Verified with 500/abort on all four; no empty-season copy or endless skeleton. |
| BUG-201 | ✅ **Fixed** | Standings chart and stats aggregates now render inline errors with Try again instead of “empty”/“no matches”. Verified with 500s on `/api/chart` and the aggregate endpoints. |
| BUG-300 | ✅ **Fixed** | Import/schedule/start map errors with the real connection state. Verified disconnected `mockforbidden` → “Connect your Spotify account…” instead of the connected-account message. |
| BUG-302 | ✅ **Fixed** | Re-import only rebuilds artist links for present tracks, preserves retained genres, and positions retained tracks after the max imported position. Verified: removed row kept `Indie/Alternative` + 2 artist links, position 28, no duplicate positions. |
| BUG-305 | ✅ **Fixed** | A 401 from a user token forces one refresh-token grant and retry. Verified with `access_token=invalid-user-token` and a future expiry: import returned 200 and imported 25 tracks. |
| BUG-110 | ✅ **Fixed** | Transport/DNS/parse and upstream 5xx errors now map to 503 with a safe message. Verified mock 500 → `503 Spotify is temporarily unavailable`. |
| BUG-250 | ✅ **Fixed** | Admin Voters/Standings/Playlist/Invites show a single error block with Try again and “Unavailable”, never an empty-state claim. Verified with 500s on all four. |
| BUG-251 | ✅ **Fixed** | Removed the danger-zone remount so success notices survive the refresh. Verified reset → “Reset complete — 270 picks and 291 duels removed.” |
| BUG-252 | ✅ **Fixed** | Preview backfill now polls the playlist overview, reports completion/still-missing, and disables the button while pending. Verified with mocked 18→0 response: “Preview fetch complete”. |
| BUG-253 | ✅ **Fixed** | Playlist panel loads 50 rows with a total and Load more, plus per-row “No preview”. Verified 50/150 → 100/150. |
| BUG-254 | ✅ **Fixed** | Revoke is now shown for every unused invite, including expired. Verified the seeded expired code has a Revoke control. |
| OPS-001 | ✅ **Fixed** | Backend healthcheck now GETs `/readyz` and requires HTTP 200. Verified container health `healthy` with the readiness HTTP check. |
| OPS-002 | ✅ **Fixed** | Added `stop_grace_period: 200s`. Verified compose resolves it to `3m20s`. |
| OPS-003 | ✅ **Fixed** | `DATABASE_URL` can now be supplied directly and overrides the interpolated default, so URL-reserved password characters are avoidable. Verified override resolution. |
| OPS-004 | ✅ **Fixed** | Backups write to `.tmp`, run `gzip -t` and a non-empty check, then atomically `mv`; failed runs clean up. Verified an actual backup run produced a valid gzip (38 kB). |

## Tier 1 & 2 status (fixed)

| ID | Status | Fix and verification |
|---|---|---|
| BUG-151 | ✅ **Fixed** | Added a monotonic `votes.seq` (migration `0016`) assigned after track locks; undo, personal replay and rebuilds now replay by `seq`. Verified 6 concurrent votes → 0 ledger mismatches, and admin clear-votes rebuild matched the remaining `seq` replay exactly. |
| BUG-001 | ✅ **Fixed** | The current duel is cached per user in `sessionStorage` and restored on mount/refresh/navigation instead of issuing a new `GET /api/matchup`. Verified reload and away/back keep the same pair and the skipped count unchanged. |
| SEC-101 | ✅ **Fixed** | Shared CSV escaping now prefixes formula-leading `= + - @ TAB CR` values with `'`; users CSV uses the same helper. Verified exported `=HYPERLINK(...)` starts with `'`. |
| OPS-005 | ✅ **Fixed** | Restore stops the backend, restores inside the maintenance window, and restarts via an EXIT trap. Verified against the isolated stack: backend stopped/restarted and baseline (150 tracks/270 votes) restored. |
| OPS-008 | ✅ **Fixed** | `SpotifyClient` now has connect/total request timeouts; scheduler start/reveal work is bounded. Code review + build pass. |
| UX-012 | ✅ **Fixed** | Mobile duel sides now show a visible “Pick A/B” chip inside the pick target; desktop keeps the existing buttons. Verified 2 visible chips at 390px. |
| UX-002 | ✅ **Fixed** | Recent-duels height is capped at 190px at all widths. Verified pick buttons bottom 649px at 1024×768 and 781px at 1440×900; auto-preview bottom 731/863px, all above the fold. |
| A11Y-004 | ✅ **Fixed** | Focus outline now uses `--playlist-accent-strong`; measured `#915b2a` (4.89:1 on paper). |
| A11Y-005 | ✅ **Fixed** | Input/search/select borders now use `--color-muted` instead of `--color-line`, exceeding 3:1. |
| A11Y-006 | ✅ **Fixed** | Removed opacity dimming from facet/filter counts, counter hints, duel metadata/listen time, and placeholders. axe is clean on `/`, `/duel`, `/stats`, `/your-duels`. |
| BUG-301 | ✅ **Fixed** | Scheduler and metadata failures now store a mapped safe message instead of raw Spotify bodies. Verified failed metadata run → “Spotify is temporarily unavailable…”. |
| BUG-100 | ✅ **Fixed** | API middleware normalises extractor/method rejections to `{"error":"..."}` JSON; 405 keeps `Allow`. Verified malformed JSON/query/path → 400 JSON, 405 JSON, 413 JSON. |

## Final polish status (fixed)

| ID | Status | Fix and verification |
|---|---|---|
| BUG-007 | ✅ **Fixed** | Clippy auto-fixes plus targeted cleanups (auto-deref, iterator, pattern, `is_multiple_of`, redundant closure) and a documented `too_many_arguments` allowance. `cargo clippy --all-targets -- -D warnings` now exits 0; 37/37 tests pass. |
| UX-001 | ✅ **Fixed** | Hover styling is scoped to inactive admin-nav items; the active item keeps paper text. Verified computed active color `rgb(247,238,223)`, inactive hover `rgb(87,82,70)`. |
| A11Y-007 | ✅ **Fixed** | Removed the custom row `aria-label`; visible row content plus an `sr-only` Show/Hide stats label now form the accessible name. axe reports **0** `label-content-name-mismatch` on `/standings` and `/your-ranking`. |
| PERF-002 | ✅ **Fixed** | IANA timezone names are loaded once into a `OnceLock<HashSet<_>>` instead of scanning `pg_timezone_names` per request. Verified summary with tz **3.9 ms p50** vs 4.4 ms without (was 37.9 ms); matchup 7.9 ms vs 8.8 ms. |

### Fix implementation notes

- **SEC-001:** `backend/src/config.rs` — `validate_admin_password` (12+ chars, placeholder and username checks) + unit tests.
- **SEC-100:** `backend/src/api/auth.rs` — `valid_login_username` / `valid_login_password` before Argon2, audit and WARN logging + unit tests.
- **BUG-150:** `backend/src/api/vote.rs` — undo deletes the vote, then replays with `ratings::rebuild_playlist_ratings`; removed delta subtraction and stale ledger fields.
- **PERF-001:** `backend/src/api/vote.rs` — `lock_track` now uses `FOR NO KEY UPDATE` with an explanatory comment.
- **A11Y-001:** `VotePage.tsx`, `DuelSide.tsx`, `ChartPage.tsx`, `AuthCard.tsx` — focus restoration after vote/undo, exhausted state, Load more and auth errors.
- **A11Y-002:** `frontend/src/index.css` — loser opacity removed from text containers; only cover art remains dimmed.
- **A11Y-003:** `frontend/src/index.css` — focus/scroll padding for the fixed bottom nav; short-viewport duel padding.
- **PERF-003:** `App.tsx` (`#main` min-height), `PlaylistBadge.tsx` (stable loaded/skeleton height), `StatsPage.tsx` (hold the explore section until stories settle), `index.css`.
- **SEC-102:** `backend/src/api/auth.rs` — logout increments `session_version` and clears the auth cache.
- **BUG-101:** `backend/src/error.rs` + chart/aggregate/ranking/dashboard/streak — control-character validation before SQL; NUL timezones fall back to UTC.
- **BUG-152:** `backend/src/api/vote.rs` — undo joins `seen_pairs.served_flip` and preserves the served A/B order.
- **BUG-200 / BUG-201:** `SeasonUnavailable.tsx` + Home/Chart/Stats/Vote pages; Chart/Stats inline error branches with retry.
- **BUG-300 / BUG-305 / BUG-110:** `spotify_auth.rs`, `api/playlist.rs`, `api/spotify.rs`, `metadata_backfill.rs` — real connection state in error mapping, forced refresh on user-token 401, safe 502/503 upstream errors.
- **BUG-302:** `metadata.rs` (only rebuild present tracks' artist links) and `api/playlist.rs` (retained-track positions after max imported position).
- **BUG-250–BUG-254:** `AdminUsers`, `AdminRankingTable`, `AdminPlaylist`, `AdminInvites`, `AdminPage`, `PlaylistOverview`/types — section error states, stable danger notice, preview polling, paginated tracks, expired-invite revoke.
- **OPS-001–OPS-004:** `compose.yaml` (HTTP `/readyz` healthcheck, `stop_grace_period`, `DATABASE_URL` override), `scripts/backup.sh` (temp + `gzip -t` + atomic move).
- **BUG-151:** `migrations/0016_vote_sequence.sql`, `ratings.rs`, `ranking.rs`, `vote.rs` — monotonic vote `seq` and `seq`-ordered replay.
- **BUG-001:** `VotePage.tsx` — per-user `sessionStorage` restore of the current duel.
- **SEC-101:** `api/chart.rs` — formula-prefix neutralisation in `csv_field`; `api/dashboard.rs` reuses it.
- **OPS-005 / OPS-008:** `scripts/restore.sh` maintenance stop/start/trap; `spotify.rs` HTTP timeouts.
- **UX-012 / UX-002:** `DuelSide.tsx`, `index.css`, `VotePage.tsx` — visible mobile pick chip, recent-duels cap at all widths.
- **A11Y-004–A11Y-006:** `index.css`, `TrackFilters.tsx`, `StatsPage.tsx`, `DuelHistoryPage.tsx` — strong focus outline, darker field borders, no low-opacity text.
- **BUG-301:** `scheduler.rs`, `metadata_backfill.rs` — safe persisted Spotify error messages.
- **BUG-100:** `main.rs` — API JSON error-normalisation middleware.
- **BUG-007:** backend-wide clippy fixes (`auth.rs`, `api/*`, `spotify.rs`) + documented `load_chart` argument allowance.
- **UX-001:** `index.css` — hover colour scoped to inactive `.pv-admin-nav-item`; active hover keeps paper text.
- **A11Y-007:** `ChartPage.tsx`, `RankingPage.tsx` — remove row `aria-label`, add `sr-only` Show/Hide stats.
- **PERF-002:** `api/streak.rs` — process-wide timezone-name cache.
- Verification run: `npx tsc --noEmit` pass; `vite build` pass; `cargo test --all-targets` **37/37 pass**; isolated stack rebuilt with `docker compose up -d --build`; mixed-load and keyboard/axe/CLS checks above.

## Still TBD

The remaining work is **15 medium + 82 low** items:

**Accessibility:** `A11Y-008` auth mode state · `A11Y-009` admin nav active state · `A11Y-010` admin chart target size · `A11Y-011` scrollable admin regions · `A11Y-012` missing main landmark · `A11Y-013` stats heading order · `A11Y-014` duel control names.

**Performance:** `PERF-004` fonts/LCP · `PERF-005` bundle/drawer duplication · `PERF-006` DB pool · `PERF-007` replay caching · `PERF-008` heavy-work semaphore · `PERF-009` history payload.

**Visual:** `UX-005` genre labels · `UX-007` mobile row density.

## Executive summary

The core game works. The backend Elo maths, streak/timezone logic, reveal gating, authz matrix, JWT validation and SQL-injection defenses verified clean. Invite-only auth, duels, picks, undo, streaks, personal ranking, sealed/revealed standings, stats, admin controls, CSV, Spotify import/backfill and scheduling all have substantial happy-path coverage. The frontend type-checks and builds; 35/35 backend tests pass.

The release risks are concentrated in seven places:

1. **Session/secret handling** — a one-character production admin password is accepted; failed-login usernames are unbounded (1.5 MB stored and logged per request); logout does not revoke a stolen JWT.
2. **Ranking integrity** — undo subtracts deltas while admin rebuilds replay votes; the same remaining picks produce different standings, and the audit ledger no longer chains. Concurrent vote order makes rebuild non-deterministic.
3. **Duel UX/correctness** — undo can mirror the card; a page load/refresh burns a duel; tiny playlists can strand a voter; a stale status poll can show “no duels” instead of “paused”.
4. **Failure states** — a season-status outage shows “no tape/no stats” and an endless duel skeleton; list-fetch 500s render as empty results with no local retry; extractor errors break the JSON error contract.
5. **Accessibility** — keyboard focus is lost after every pick/undo/load-more; losing-side history text is 1.95–3.31:1; the fixed bottom nav covers focused controls; focus rings and field borders also fail non-text contrast.
6. **Performance under load** — mixed vote/matchup traffic deadlocks and returns 500s at 30 voters; desktop CLS is 0.65–1.60; fonts are 59 % of page weight; the 16-connection pool saturates at 100 voters.
7. **Release hygiene** — `cargo fmt --check` and `cargo clippy -D warnings` fail; the shipped nginx config has no TLS/HSTS; backup/restore and container hardening need attention.

## Coverage at a glance

| Area | What ran |
|---|---|
| Static/build | `tsc --noEmit`, `vite build`, release build, 35/35 tests, 15 migrations on a fresh DB, route/README/`types.ts` comparison, dependency/secret scan |
| API | 53 paths / 55 method+path operations × anonymous/voter/admin; validation, CSV, caching, compression, rate limits, contract |
| Security | headers/cookies/JWT, full authz matrix, IDOR, CSRF, XSS/SQLi/path traversal/SSRF, CSV, DoS, secrets, Docker |
| Domain | Elo, matchup, vote/undo, concurrency, streaks/DST, reveal/seal, scheduling, rebuilds, DB invariants |
| E2E voter | ~119 scripted journeys / ~260 assertions across desktop, mobile and landscape; auth, duel, history, ranking, standings, stats, failure injection |
| Admin | all 9 admin sections, destructive actions, CSV, voter management, settings, reveal/pause, invites; 65-user pagination fixture |
| Visual | 1440/1920/1024/820/390/320 widths, 200% zoom, sealed/revealed, empty/error/loading/long-content states (final pass) |
| Accessibility | 64 axe runs on ~62 page/state/viewports, ~6,075 text elements contrast-checked, keyboard/focus, reflow, AX tree |
| Performance | 16 Lighthouse runs, 26 endpoints, 10–100-voter concurrency, DB plans at 150/2,000 tracks, dump/restore |
| Spotify | deterministic mock: import, pagination, dedupe, metadata, previews, OAuth, refresh, scheduling, error mapping |

## High — release blockers (all fixed)

> All eight findings below are **fixed and verified** on the rebuilt stack; see the release-blocker table above for the exact fix and verification. The detail remains for traceability.

### SEC-001 — Production accepts a one-character admin password and creates the account
- **Severity:** high (verified on frozen tree; production-mode process)
- **Area:** security / configuration
- **What:** `APP_ENV=production` only rejects the exact placeholders and the literal `admin`; there is no minimum length or complexity check for `ADMIN_PASSWORD`. Starting with `ADMIN_PASSWORD=a` succeeded, bootstrapped an admin, and `POST /api/auth/login` returned 200 with that password.
- **Why it matters:** the README promises production refuses weak admin credentials. A one-character admin password gives full mission-control, destructive reset and voter-management access.
- **Fix:** apply the password policy (preferably 12+ characters) to bootstrap credentials; also validate `ADMIN_USERNAME`; fail startup with a clear message.
- **Evidence:** `/tmp/qa/static/backend-prod-run.log`, `/tmp/qa/static/weaklogin.json`; reproduced independently on the frozen image.
- **Location:** `backend/src/config.rs:113-120`, `backend/src/main.rs:203-237`.

### SEC-100 — Failed-login username is persisted and logged without a length bound [SEC-103]
- **Severity:** high (verified on frozen tree; unauthenticated)
- **Area:** security / availability / audit integrity
- **What:** `POST /api/auth/login` accepts a ~1.5 MB username. The full value is written to `auth_events.username` and to a WARN log line. One request inserted a 1,500,000-character value; at the default 30 attempts/min/IP this is tens of MB of persistent DB and log growth per minute. Newlines in the username also forge independent log lines.
- **Why it matters:** unauthenticated disk/log exhaustion, loss of audit history through log rotation, and forged log entries. Nginx caps the body at 1 MB in production, which still leaves a large attack surface.
- **Fix:** apply `validate_username` (or a 1–32 length/charset check) before the rate-limit/audit path; truncate/escape audit and log fields; add `auth_events` retention; consider JSON logs.
- **Evidence:** DB `max(length(username))=1500000`, `auth_events` grew by ~1.5 MB; log line split by newline. `/tmp/qa/findings/security.md` SEC-100/SEC-103.
- **Location:** `backend/src/api/auth.rs:login`, `backend/src/audit.rs:record`.

### BUG-150 — Undo and admin rebuild produce different standings for the same remaining picks [BUG-350]
- **Severity:** high (verified; data integrity)
- **Area:** ranking / voting / audit
- **What:** Undo subtracts the removed vote’s stored deltas; admin clear/delete replays surviving votes from scratch. In a controlled 3-track/3-voter scenario, the same remaining picks produced A=1039.726/B=960.274 after undo versus A=1037.486/B=962.514 after rebuild — a 2.24 Elo difference. The surviving vote still records before-values that no longer match the official ratings, so the audit ledger does not chain. Undo followed by re-picking the same side does not restore the original score.
- **Why it matters:** the official standings become path-dependent and can be nudged by undo/re-pick cycles; the admin “rebuild” cannot reproduce the public ranking.
- **Fix:** after deleting the vote in `undo`, call `ratings::rebuild_playlist_ratings` inside the same transaction (it already exists and locks tracks in UUID order), or document that undo is approximate and stop presenting rebuild as the source of truth.
- **Evidence:** `/tmp/qa/tmp/undo_vs_clear_clean.txt`, `/tmp/qa/tmp/undo_replay_evidence.txt`; independently reproduced.
- **Location:** `backend/src/api/vote.rs:304-327`, `backend/src/ratings.rs:54-250`.

### A11Y-001 — Keyboard focus is lost after every vote, undo, load-more and auth submit
- **Severity:** high (verified on frozen build)
- **Area:** a11y / keyboard
- **What:** The focused pick/undo/submit control becomes `disabled` or is removed while busy, so Chromium moves focus to `<body>`. After a vote on `/duel`, a keyboard user must press Tab up to 21 times to reach the next card. The same happens after Undo, “Load more” on `/standings`, and the sign-in submit.
- **Why it matters:** WCAG 2.2 SC 2.4.3 Focus Order / SC 2.1.1 Keyboard; the core voting loop is impractical without a mouse.
- **Fix:** use `aria-disabled` + handler guard instead of `disabled`; after a vote/undo focus the first pick button of the new matchup (or a `tabindex="-1"` progress heading); keep “Load more” mounted and focus the first new row/status; focus the first invalid field on submit error.
- **Evidence:** `/tmp/qa/tmp/p2-interact.json` (`duel.at150/after/undoAfter/loadMore/authError` all `BODY`); `/tmp/qa/tmp/p2-interact.log`.
- **Location:** `VotePage.tsx:656,716,795,818`, `DuelSide.tsx`, `DuelStatus.tsx:50-63`, `ChartPage.tsx:815-826`, `AuthCard.tsx:171`.

### A11Y-002 — Losing-side text in duel history/recent duels is below 4.5:1 [UX-011]
- **Severity:** high (verified on frozen build)
- **Area:** a11y / contrast
- **What:** `.pv-history-side.is-loser` uses `opacity: 0.5` (title 3.31:1, artist 2.33:1) and `.pv-recent-side.is-loser` uses `opacity: 0.42` (title 2.59:1, artist 1.95:1). The loser’s title/artist is meaningful content, not disabled UI. axe reports 94 contrast nodes on `/your-duels` desktop and 92 on mobile.
- **Why it matters:** WCAG 2.2 SC 1.4.3; the history is hard to read for low-vision users, especially on mobile.
- **Fix:** remove opacity from text-bearing containers; distinguish the loser with a border/tint/icon and keep title/artist at ≥4.5:1. Dim only decorative cover art if desired.
- **Evidence:** `/tmp/qa/tmp/p2-contrast.json`, `p2-axe.json` (`color-contrast x94`), `/tmp/qa/shots/a11y/p2-duel-390x844.png`.
- **Location:** `frontend/src/index.css` `.pv-history-side.is-loser`, `.pv-recent-side.is-loser`; `HistoryDuelCard.tsx:164-171`, `RecentDuels.tsx:13`.

### A11Y-003 — Fixed bottom navigation obscures keyboard-focused controls on mobile
- **Severity:** high (verified on frozen build)
- **Area:** a11y / keyboard / mobile
- **What:** On `/your-duels` at 390×844, focus stops at `top: 784, bottom: 828` while the fixed Primary nav starts at 779; eight consecutive preview buttons are fully covered, and `elementFromPoint` returns the nav. In 844×390 landscape the second pick button extends under the nav.
- **Why it matters:** WCAG 2.2 SC 2.4.11 Focus Not Obscured (Minimum); the focus ring is invisible and the control cannot be activated.
- **Fix:** add `scroll-padding-bottom`/`scroll-margin-bottom` for the fixed nav, or make the nav non-fixed at short heights; reserve nav height in the duel landscape layout.
- **Evidence:** `/tmp/qa/tmp/p2-interact.json` (`obscured`, 8 covered entries), `p2-heights.json` (`navTop 325`, pick bottom 352), `/tmp/qa/shots/a11y/p2-focus-obscured-nav.png`.
- **Location:** `BottomTabs.tsx:134-137`, `index.css` `.pv-duel-page`.

### PERF-001 — Concurrent voting deadlocks and returns 500s
- **Severity:** high (verified under mixed load)
- **Area:** performance / voting / reliability
- **What:** With 30 voters looping matchup→vote for 20 s, 145 vote requests produced 3×500 and 150 matchups produced 5×500, with 8 `deadlock detected` errors; vote p95 rose to 1,026 ms (vs 86 ms for a pure 50-vote burst). Pure vote bursts do not fail, so this is a cross-endpoint lock cycle: vote holds `FOR UPDATE` on a track, `INSERT INTO seen_pairs` waits for the FK `KEY SHARE` on the other track, and the vote waits for the `seen_pairs` transaction.
- **Why it matters:** at launch, voters will see random 500s and multi-second stalls; this is the most important reliability finding.
- **Fix:** use `FOR NO KEY UPDATE` in `lock_track` (no key columns change), retry SQLSTATE `40P01` 2–3 times with jitter, or insert `seen_pairs` outside the matchup transaction.
- **Evidence:** `/tmp/qa/shots/perf/raw/loadlogs_final_mixed.txt`; Postgres deadlock detail.
- **Location:** `backend/src/api/vote.rs:391-408`, `backend/src/api/matchup.rs:100-111`, `migrations/0004_matchups.sql`.

### PERF-003 — Layout shift of 0.65–1.60 as the footer is pushed below the fold
- **Severity:** high (verified on frozen build)
- **Area:** performance / visual
- **What:** Desktop CLS is 0.746–1.596 on every data page; mobile admin 1.020, stats 0.962, your-ranking 0.524, standings revealed 0.347. Playwright attributes the largest shift to `#main`/footer: the initial shell fits the viewport with the footer at y≈816; when data arrives the footer is pushed far below (source rect delta 672–816 px). Stats has a second wave as story/aggregate sections insert.
- **Why it matters:** desktop Lighthouse is capped at 75–77 despite LCP 0.63–0.76 s / TBT 0; the page visibly jumps during load.
- **Fix:** reserve `#main` height (`min-height: calc(100svh - header)`) so the footer starts below the fold; make skeletons match final block heights; hide/absolutely position the footer until the first data load settles.
- **Evidence:** `/tmp/qa/shots/perf/raw/lh_final_*`, `cls_new.json`, `/tmp/qa/shots/perf/*-cls.png`.
- **Location:** `frontend/src/App.tsx:44-61`, `StatsPage.tsx`, `ChartPage.tsx`, `RankingPage.tsx`.

## Medium — fix or explicitly accept

> 34 of the 49 medium findings are fixed (see the fixed-findings tables); **15 remain TBD**.

### SEC-101 — CSV formula injection in the standings export [SEC-002, BUG-102]
- **Severity:** medium (verified)
- **Area:** security / admin
- **What:** `GET /api/admin/chart.csv` quotes commas/quotes/newlines but does not neutralise cells beginning `=`, `+`, `-`, `@`, tab or CR. The fixture title `=HYPERLINK("http://example.invalid/","click me")` is exported and evaluates in Excel/LibreOffice/Sheets.
- **Why it matters:** a shared/collaborative Spotify playlist can inject a formula that the admin later opens as a downloaded attachment.
- **Fix:** prefix dangerous leading characters with `'` in the shared CSV helper; add regression cases for `= + - @ TAB CR`.
- **Evidence:** `/tmp/qa/tmp/security/chart.csv`; parsed CSV row 42.
- **Location:** `backend/src/api/chart.rs:csv_field` (and `dashboard.rs`).

### SEC-102 — Logout does not revoke the JWT [SEC-003, BUG-103]
- **Severity:** medium (verified)
- **Area:** security / sessions
- **What:** Logout only clears the browser cookie. A captured `pv_session` cookie remains valid for the full 7-day TTL. Admin force-logout, password reset and disable do revoke correctly via `session_version`.
- **Why it matters:** a stolen token survives the user’s sign-out; users have no self-service way to revoke it.
- **Fix:** bump `session_version` on logout (logs out all devices) or add a `jti` denylist/short-lived access token plus refresh rotation.
- **Evidence:** replay after logout returned 200; `/tmp/qa/findings/security.md`.
- **Location:** `backend/src/api/auth.rs:logout`.

### BUG-100 — Extractor rejections bypass the JSON error envelope [BUG-006, SEC-107, BUG-310]
- **Severity:** medium (verified)
- **Area:** API contract / UX
- **What:** Malformed JSON, missing fields, bad path UUIDs and invalid query values return `text/plain` axum/serde messages (400/415/422), not `{"error":"..."}`. 405 responses are empty. The frontend therefore shows “request failed with status 422/400/415” instead of the real reason.
- **Why it matters:** the documented error contract is violated, error messages leak parser internals, and clients cannot render useful errors.
- **Fix:** wrap `Json`, `Query` and `Path` extractors (or map their rejections) to return the standard JSON envelope; normalise 422→400; add a 405 method fallback with `Allow` and JSON.
- **Evidence:** `/tmp/qa/findings/api.md` (exact status/content-type/body for each case).
- **Location:** `backend/src/main.rs:282-289`, `backend/src/error.rs`.

### BUG-101 — NUL bytes in search/login trigger a 500
- **Severity:** medium (verified)
- **Area:** API / robustness
- **What:** `q=%00`, `genre=%00`, or a `\u0000` login username reaches PostgreSQL and raises `invalid byte sequence for encoding "UTF8": 0x00`, returned as generic 500. Publicly reachable on `/api/chart` after reveal; login is unauthenticated.
- **Why it matters:** a client-controlled 500 is noisy, logs stack-level errors and breaks the “client errors are 4xx” contract.
- **Fix:** reject NUL/control characters in a shared input validator or map the DB encoding error to 400.
- **Evidence:** `/tmp/qa/findings/api.md`; server log excerpt.
- **Location:** `backend/src/api/auth.rs`, `backend/src/api/chart.rs`, `aggregate.rs`, `dashboard.rs`.

### BUG-001 — Loading `/duel` mutates state and silently burns a duel on every refresh
- **Severity:** medium (verified)
- **Area:** voting / retention
- **What:** `GET /api/matchup` inserts into `seen_pairs` before returning. Every mount/refresh creates a new skipped duel. A single `GET` produced `skipped:0→1`; refreshing repeatedly permanently removes pairs and increments the skipped count.
- **Why it matters:** an accidental refresh costs a duel, changes coverage and pollutes “No pick” history; on tiny playlists it can strand the voter (see BUG-155).
- **Fix:** cache the served pair in `sessionStorage` and restore it on mount, or split “serve” from “record” so only a deliberate next/skip action records the pair.
- **Evidence:** `/tmp/qa/static/runtime-observations.md`; independently reproduced (`skipped 0 → 1`).
- **Location:** `backend/src/api/matchup.rs:100-112`, `frontend/src/pages/VotePage.tsx`.

### BUG-151 — Rebuild replays votes in transaction-start order, not the order they were applied
- **Severity:** medium (verified under concurrency)
- **Area:** ranking / data integrity
- **What:** `votes.created_at` defaults to `now()` (transaction start, before track locks). Concurrent votes can therefore be ordered differently from the lock/commit order. A rebuild by `created_at` rewrites surviving voters’ deltas; 3/6 votes mismatched in the first round. A later-created vote stored `1000/1000` before-values while an earlier-created vote already had non-initial values.
- **Why it matters:** every admin clear/delete rebuild can shift standings even when those voters’ picks were not removed.
- **Fix:** add a `BIGSERIAL seq` assigned after locks and replay by `seq`, use `clock_timestamp()` at insert, or serialise votes per playlist.
- **Evidence:** `/tmp/qa/tmp/order_race_out.txt`, `/tmp/qa/tmp/order_race.py`.
- **Location:** `backend/src/api/vote.rs:146-225`, `migrations/0005_votes.sql:21`, `backend/src/ratings.rs:133-139`.

### BUG-152 — Undo re-shows the undone duel with the sides swapped [BUG-012, BUG-351]
- **Severity:** medium (verified)
- **Area:** duel / UX
- **What:** `undo` returns `track_a = low UUID`, `track_b = high UUID`, ignoring `seen_pairs.served_flip`. About 53% of undos return the card mirrored; the frontend replaces the matchup with that response, so a positional/keyboard re-pick can choose the other track. History honors `served_flip`, so history and the undo screen disagree.
- **Why it matters:** a user can record a vote for the wrong song immediately after undo.
- **Fix:** join `seen_pairs` in the undo query and return the original orientation (or expose `served_flip` and let the client apply it).
- **Evidence:** `/tmp/qa/tmp/undo_orientation_evidence.txt`; independently reproduced.
- **Location:** `backend/src/api/vote.rs:334-388`, `frontend/src/pages/VotePage.tsx:585-593`.

### BUG-200 — Season-status outage is shown as “no tape”/empty pages and an endless duel skeleton [BUG-003]
- **Severity:** medium (verified on frozen build)
- **Area:** error handling / availability
- **What:** If `/api/chart/status` returns 500 or aborts, the global banner appears, but Home says “NO TAPE YET” with zero counters, Standings says “The playlist isn't here yet.”, Stats says “There are no stats yet.”, and `/duel` stays on the loading skeleton. The banner Retry recovers, but the main content is misleading until then.
- **Why it matters:** a transient outage looks like data loss/empty season; duel users have no local retry.
- **Fix:** expose a distinct `statusError`/`unavailable` state and render per-page error panels with retry; never treat `status === null && error` as an empty season.
- **Evidence:** `/tmp/qa/shots/e2e/BUG-status-500-*.png`, `BUG-status-abort-home.png`.
- **Location:** `frontend/src/context/PlaylistContext.tsx`, `HomePage.tsx`, `ChartPage.tsx`, `StatsPage.tsx`, `VotePage.tsx`.

### BUG-201 — Standings/Stats list-fetch 500s render as empty results with no inline retry
- **Severity:** medium (verified on frozen build)
- **Area:** error handling / standings / stats
- **What:** With The Reveal open, a failed `/api/chart?*` or aggregate request shows the raw error in the header but the lists say “The standings are empty.”, “0 ARTISTS · 0 ALBUMS”, “No artists match these filters.”, “No albums match these filters.”, even with no filters. Only the header Refresh recovers.
- **Fix:** pass request errors into the list/aggregate components and render an error block with a local retry.
- **Evidence:** `/tmp/qa/shots/e2e/BUG-chart-500-standings.png`, `BUG-aggregate-500-stats.png`.
- **Location:** `frontend/src/pages/ChartPage.tsx`, `frontend/src/pages/StatsPage.tsx`.

### BUG-300 — Disconnected admins get the “connected account must own…” 403
- **Severity:** medium (mock-verified)
- **Area:** Spotify / admin UX
- **What:** Import/schedule hardcode `map_error(error, true)`, so a 403 with no stored credentials tells the admin to fix a connected account. The “connect your account” branch is unreachable from these routes.
- **Fix:** carry the real auth mode/connection state into error mapping.
- **Evidence:** `/tmp/qa/findings/spotify.md`; reproduced with `connected:false`.
- **Location:** `backend/src/api/playlist.rs:83,252,450`, `backend/src/api/spotify.rs:176-197`.

### BUG-301 — Raw Spotify response bodies are stored and shown in the admin UI
- **Severity:** medium (mock-verified)
- **Area:** Spotify / admin / information disclosure
- **What:** Scheduler `start_error` and failed metadata runs persist `format!("{error:#}")`, then the Playlist panel renders it. The UI can show `Spotify API error (500): {"error":{"status":500,"message":"boom"}}`.
- **Why it matters:** raw upstream payloads leak into the admin UI and database.
- **Fix:** map to safe user-facing strings; keep full detail in logs only; never persist upstream bodies.
- **Evidence:** `/tmp/qa/shots/spotify/12-scheduler-error-leak.png`; `latest_run.error` sample.
- **Location:** `backend/src/scheduler.rs:111`, `backend/src/metadata_backfill.rs:97-105`, `frontend/src/components/AdminPlaylist.tsx`.

### BUG-302 — Re-import strips artist/genre data from tracks removed from Spotify, and collides positions [BUG-354]
- **Severity:** medium (mock-verified)
- **Area:** playlist / metadata / stats
- **What:** The removed track row is intentionally retained, but `sync_track_artists` deletes every `track_artists` row for the playlist and re-links only the fresh import; `refresh_track_genres` then clears the retained track’s genre. Its old position also duplicates the next track’s position.
- **Why it matters:** the README promises removed tracks keep history, but they disappear from artist/genre facets and stats; positions become ambiguous.
- **Fix:** delete/rebuild links only for tracks in the fresh import, preserve genres for retained tracks, and reconcile positions for removed rows.
- **Evidence:** `mocktrack…117` before `119|Folk/Country|2`, after `119|NULL|0`; collision query `119|2`.
- **Location:** `backend/src/metadata.rs:227-263,330-357`, `backend/src/api/playlist.rs:788-833`.

### BUG-305 — A 401 from a user access token is never refreshed and is reported as bad app credentials
- **Severity:** medium (mock-verified)
- **Area:** Spotify / OAuth
- **What:** When a stored user access token is rejected with 401 while `expires_at` is still in the future, the import returns 500 and logs “Spotify rejected the configured app credentials”; no refresh-token grant is attempted even though the refresh token is valid.
- **Why it matters:** a recoverable token-expiry edge becomes a dead-end admin error until manual reconnect.
- **Fix:** on 401 with a user token, force one refresh and retry; map refresh `invalid_grant` to Reconnect; reserve “app credentials” for client-credentials failures.
- **Evidence:** `token calls 0`, import 500, status still `connected:true`.
- **Location:** `backend/src/spotify.rs:487-493`, `backend/src/spotify_auth.rs:41-46`, `backend/src/api/spotify.rs:201-204`.

### BUG-110 — Spotify/upstream failures surface as a generic 500, including admin start/schedule [BUG-255, BUG-352]
- **Severity:** medium (mock/network-verified)
- **Area:** Spotify / admin / error handling
- **What:** Import, schedule and start map transport/DNS/upstream 5xx failures to `ApiError::Internal`, so the admin sees `500 {"error":"internal server error"}`. The useful `start_error`/log detail only appears after a manual refresh. A disconnected 403 can also use the wrong “connected account” message (`BUG-300`).
- **Why it matters:** an operational outage looks like an application bug; admins cannot tell whether to reconnect, wait, or fix DNS.
- **Fix:** add a `SpotifyError::Network/Unavailable` mapped to 502/503 with a friendly message; return safe detail; reload the playlist panel in the `catch` so `start_error` is visible.
- **Evidence:** `/tmp/qa/shots/admin/playlist-start-failure.png`, `/tmp/qa/shots/admin/playlist-schedule-failure.png`; log `failed to reach Spotify accounts API`.
- **Location:** `backend/src/api/spotify.rs:map_error`, `backend/src/api/playlist.rs`.

### BUG-250 — Admin section load failures also render empty-state and “Loading…” copy
- **Severity:** medium (verified on frozen build)
- **Area:** admin / error handling
- **What:** A 500 from Voters, Standings, Playlist or Invites renders the red error banner **together with** “No voters yet”, “No tracks in this playlist”, “No playlist imported yet” or “No invite codes yet”; the Voters subtitle stays “Loading…”. An admin can read a failed request as real emptiness and proceed to a destructive reset/remove.
- **Fix:** gate empty/loading copy on `!error`; render one error state with a section-level Retry.
- **Evidence:** `/tmp/qa/shots/admin/error-voters.png`, `error-standings.png`, `error-playlist.png`, `error-invites.png`.
- **Location:** `AdminUsers.tsx`, `AdminRankingTable.tsx`, `AdminPlaylist.tsx`, `AdminInvites.tsx`.

### BUG-251 — Danger-zone success notices never render
- **Severity:** medium (verified)
- **Area:** admin / UX
- **What:** `setNotice(...)` runs immediately before the parent increments `refreshKey`, which remounts `AdminDangerZone`; the success notice is lost. Data outcomes are correct, but the only feedback is silently changed counters.
- **Fix:** keep the notice in the parent/toast layer, or refetch without remounting the danger panel.
- **Evidence:** `/tmp/qa/shots/admin/danger-after-reset.png`, `danger-after-remove.png`, `danger-after-full-reset.png`.
- **Location:** `AdminDangerZone.tsx:40-96`, `AdminPage.tsx:208-214`.

### BUG-252 — Preview backfill says “Fetching…” forever and re-enables immediately
- **Severity:** medium (verified)
- **Area:** playlist / admin
- **What:** The green “Fetching 18 missing previews…” notice never resolves. The button re-enables instantly and can spawn overlapping runs; the backend finished `updated=0 missed=18`, but the UI has no result/progress state.
- **Fix:** expose a preview-backfill run id/progress, poll it, replace the optimistic notice with final counts, and disable the button while running.
- **Evidence:** `/tmp/qa/shots/admin/playlist-preview-backfill.png`; log `preview backfill finished total=18 updated=0 missed=18`.
- **Location:** `AdminPlaylist.tsx:302-319,565-569`.

### BUG-253 — Playlist shows only 6 of 150 tracks with no pagination [BUG-309]
- **Severity:** medium (verified)
- **Area:** admin / playlist
- **What:** `getTracks(6)` is called once; only six rows render, with no pagination/expand control even though the API supports `limit/offset` and reports 150. Preview gaps are only a nav hint (“18 gaps”), with no rows/counts identifying them.
- **Fix:** add limit/offset pagination or “show all” with virtualisation; surface preview presence/missing rows.
- **Evidence:** `/tmp/qa/shots/admin/section-playlist-1440.png`, `frozen-playlist.png`; DOM rows 6, pagination 0.
- **Location:** `AdminPlaylist.tsx:79-87,520-539`, `lib/api.ts:188-189`.

### BUG-254 — Expired unused invites cannot be revoked from the UI
- **Severity:** medium (verified)
- **Area:** admin / invites
- **What:** “Revoke” renders only when `status === 'open'`; an expired but unused code offers only “Copy code”, although `DELETE /api/admin/invites/{id}` returns 200. Admins cannot clean up expired codes.
- **Fix:** render Revoke for every unused invite (`status !== 'used'`), including expired.
- **Evidence:** `/tmp/qa/shots/admin/invites-expired.png`; button list `["COPY CODE"]`.
- **Location:** `AdminInvites.tsx:14-20,192-209`.

### OPS-001 — Backend healthcheck only proves the TCP port is open
- **Severity:** medium (code review)
- **What:** Compose uses `bash -c '</dev/tcp/127.0.0.1/8080'`; a DB outage leaves the container “healthy”. `/readyz` is never used.
- **Fix:** HTTP-check `/readyz` (add curl/wget or a tiny shell HTTP check); keep a start period.
- **Location:** `compose.yaml:54-60`.

### OPS-002 — No `stop_grace_period`; Docker’s 10s default can kill a 180s import/backfill
- **Severity:** medium (code review)
- **What:** Backend supports graceful shutdown and 180s requests, but Compose has no `stop_grace_period`; in-flight imports/background work can be SIGKILLed after 10s.
- **Fix:** set `stop_grace_period: 200s` (and/or move long work to a resumable job).
- **Location:** `compose.yaml:33-60`.

### OPS-003 — `DATABASE_URL` interpolates `POSTGRES_PASSWORD` without URL-encoding
- **Severity:** medium (code review)
- **What:** A password containing `@`, `:`, `/`, `?` or `#` breaks the composed `postgres://…` URL; the backend fails with a misleading connection error.
- **Fix:** accept a full `DATABASE_URL` or percent-encode the password; validate at startup.
- **Location:** `compose.yaml:37`.

### OPS-004 — `backup.sh` can leave a truncated `.sql.gz` that looks valid
- **Severity:** medium (code review)
- **What:** gzip writes directly to the final backup path; an interrupted dump leaves a partial file, and the retention `find` can later delete older good backups. No `gzip -t`/restore verification.
- **Fix:** dump to a temp file with a trap, `gzip -t`, then atomic `mv`; rotate only after success.
- **Location:** `scripts/backup.sh:8-16`.

### OPS-005 — Restore runs against a live backend
- **Severity:** medium (code review)
- **What:** `restore.sh` pipes `psql --single-transaction` into the live DB while the backend keeps serving writes; concurrent votes/imports can block or roll back the restore.
- **Fix:** stop the backend, wait, restore, then start; document maintenance downtime.
- **Location:** `scripts/restore.sh:16-23`.

### OPS-008 — Scheduler has no outbound HTTP timeout; one stalled Spotify call blocks scheduled reveals
- **Severity:** medium (code review)
- **What:** `reqwest::Client::new()` has no timeout. `process_due_start` awaits the import before `process_due_reveal`; a hung connection delays reveals indefinitely (beyond OS TCP timeouts). Stale-start cleanup only runs after 600s.
- **Fix:** add connect/read timeouts (10–30s), wrap scheduler calls in `tokio::time::timeout`, process reveals independently.
- **Location:** `backend/src/spotify.rs:117-125`, `backend/src/scheduler.rs:46-147`.

### BUG-007 — `cargo clippy --all-targets -- -D warnings` fails with 24 errors
- **Severity:** medium (verified on frozen tree)
- **What:** 19 `explicit_auto_deref`, plus `double_ended_iterator_last`, `manual_pattern_char_comparison`, `too_many_arguments`, `redundant_closure`, `manual_is_multiple_of`. Release build succeeds, but a standard quality gate fails.
- **Fix:** run `cargo clippy --fix`, review the 8-arg function, add clippy/fmt to CI.
- **Evidence:** `/tmp/qa/frozen-backend` clippy log; `CLIPPY_EXIT=101`.
- **Location:** multiple backend files.

## Low-priority polish and hardening

| ID | Area | Issue | Suggested fix |
|---|---|---|---|
| BUG-002 | voting | A 409 from a stale status poll is treated as “all duels done” even when the real error is “voting is paused”. | Add typed error codes or inspect the message; render paused/scheduled states. |
| BUG-008 | ops | `cargo fmt --check` fails. | Run `cargo fmt`; add CI gate. |
| BUG-010 | admin | `GET /api/admin/stats` and `getDashboardStats` are shipped but unused. | Remove or wire into the overview. |
| BUG-011 | visual | `/design` ships in the production bundle and is publicly routable. | Gate to admin/dev or remove the route/chunk. |
| BUG-013 | admin | Invite list is silently capped at 200 with no pagination. | Add paging or show “latest 200”. |
| BUG-014 | visual | No OG/Twitter/canonical/apple-touch metadata; mono webfont not preloaded. | Add share metadata and preload the mono weight. |
| BUG-104 | API | 405 responses are empty and non-JSON. | Add a JSON method-not-allowed fallback preserving `Allow`. |
| BUG-105 | API | `/api/` and wrong-case `/API/...` return the SPA HTML with 200. | Explicit `/api` catch-all returning JSON 404. |
| BUG-106 | ops | Missing hashed assets return 404 with `immutable, max-age=1y`. | Apply immutable cache only to 2xx; no-store 404s. |
| BUG-107 | ops | `/healthz` and `/readyz` omit `Cache-Control`. | Add `no-store`. |
| BUG-108 | ops | Client `X-Request-Id` is echoed and logged unbounded. | Generate a UUID or validate/bound it. |
| BUG-109 | auth | 429 has no `Retry-After`; successful logins consume the brute-force budget. | Add `Retry-After`; count failures separately or document. |
| BUG-111 | auth | Password max is 128 **bytes** but the error says characters (emoji fail). | Use `chars().count()` or change the message. |
| BUG-112 | API | Unknown `status`/`sort` values silently fall back to defaults. | Return 400 or document the fallback. |
| BUG-113 | search | `%` and `_` are SQL wildcards, not literal search characters (also admin voters/standings) [BUG-264]. | Escape `\`, `%`, `_` and use `ESCAPE`. |
| BUG-114 | auth | Invalid/expired session is treated as anonymous on `/api/tracks/{id}`. | Distinguish “no cookie” from “invalid cookie”; return 401. |
| BUG-115 | docs | README misses `previews/backfill`, `/api/health`, and misstates the rate-limit IP header. | Update the API table and proxy note. |
| SEC-004 | security | `cargo audit` reports RUSTSEC-2023-0071 (`rsa`), present via optional/unlinked SQLx MySQL code. | Track upstream; document the exception. |
| SEC-005 | security | Spotify access/refresh tokens are plaintext and included in backups. | Encrypt at rest, restrict access, revoke on disconnect. |
| SEC-104 | security | Shipped nginx is HTTP-only; HSTS is absent; `server_tokens` not disabled. | Ship HTTPS/HSTS/server_tokens config and verify certbot output. |
| SEC-105 | security | `X-Real-IP`/`X-Forwarded-For` are trusted if the backend is reachable directly; IPv6 rotation defeats buckets. | Trust only the local proxy; normalise IPv6; prune buckets. |
| SEC-106 | security | Health endpoints disclose version, environment, uptime and DB state. | Return a minimal `{"status":"ok"}` publicly. |
| SEC-108 | security | JWT has no `aud`/`iss`; environments sharing a secret accept each other’s tokens. | Unique secrets per env; add/validate `aud`/`iss`. |
| SEC-109 | security | Containers lack read-only rootfs, cap drops, `no-new-privileges`, resource limits; app uses DB superuser. | Harden Compose/Dockerfile; add a least-privilege DB role. |
| SEC-110 | security | `.env` is `0664` on the host. | `chmod 600`, ideally root-owned. |
| SEC-111 | security | Invite codes are logged at INFO and stored in `auth_events.detail`. | Mask/redact; treat codes as secrets. |
| SEC-112 | security | Unauthenticated Spotify callback performs a DB delete + WARN per invalid state, unthrottled. | Validate shape, rate-limit, debug-log. |
| SEC-113 | security | Password policy is length-only (8 chars); no breach list/admin MFA. | 12+ chars, breach check, per-account throttling, admin MFA. |
| BUG-303 | playlist | Re-import resets `started_at` to the re-import time. | `COALESCE(playlists.started_at, now())`. |
| BUG-304 | playlist | Concurrent schedule requests can hit the unique active-playlist index and return 500. | Serialize with an advisory lock or map to 409/retry. |
| BUG-306 | Spotify | After `invalid_grant`, status still says connected and there is no Reconnect action. | Clear/flag credentials and offer Reconnect. |
| BUG-307 | Spotify | Metadata backfill on a scheduled-only playlist reports a completed 25-track run while writing nothing. | Require `started_at` or import tracks first. |
| BUG-308 | ops | Reaped stale run says “the backend restarted” even when it merely timed out. | Use a stale/timeout message. |
| BUG-256 | admin | With no active playlist, standings CSV links stay active and download a JSON 404. | Hide/disable chart CSV when `playlist_id` is null. |
| BUG-257 | admin | Metadata backfill failure keeps the green “Backfilling…” notice and shows 0/0. | Clear the notice when the run fails; show a failed summary. |
| BUG-258 | admin | Voters management panel collapses after every action. | Keep expanded row state above the refresh boundary. |
| BUG-259 | admin | Settings goal does not save on Enter; unsaved value remains visible. | Wrap in a form/onSubmit; indicate unsaved state. |
| BUG-260 | admin | Status strip says “No tape loaded” with zeroed stats when `/admin/insights` fails. | Render “Data unavailable”/em dashes. |
| BUG-261 | admin | 32-char username truncates on mobile with no full-name affordance. | Add `title`/wrap/accessible label. |
| BUG-262 | admin | Used invite whose user was deleted is labelled “Created …”. | Render “Used by a deleted account” when `used_at` is set. |
| BUG-263 | admin | Revoked invites disappear with no revoked state or UI audit trail. | Soft-delete with `revoked_at` or record/expose history. |
| BUG-265 | admin | Danger-zone controls are clipped/wrapped; full-reset placeholder shows “Type RESET EVE”. | Widen field/card, keep labels one line, add mobile bottom padding. |
| BUG-309 | admin | Merged into `BUG-253` (playlist 6-row cap). | See `BUG-253`. |
| BUG-153 | ranking | Personal-ranking ties flip order/rank between requests (HashMap order, no id tiebreak). | Add `id ASC` as the final sort key. |
| BUG-154 | duel | `last_compared_at ASC NULLS LAST` deprioritises never-compared tracks; constants (200/40) contradict README (100). | Use `NULLS FIRST`, clear timestamp on undo, align docs/constants. |
| BUG-155 | duel | In a 2-track playlist, a served-but-unvoted pair can never be fetched again; reload strands a voter at 0 picks. | Re-serve the last unpicked pair or return actionable counters. |
| BUG-202 | copy | “1 matches” in ranking/standings. | Pluralize. |
| BUG-203 | copy | “No pick” empty state says “No skipped duels.” | Use “No unpicked duels.” |
| BUG-204 | duel | Auto-preview on a cold reload is silently blocked by autoplay policy. | Show “Tap to start” or start muted. |
| BUG-205 | duel | Space cycles to the other preview instead of pausing the current one. | Pause if playing; relabel the hint if intended. |
| BUG-206 | detail | Inline track-detail 500 shows a bare error with no retry. | Add Try again (collapse/reopen currently recovers). |
| OPS-007 | ops | Backups are local-only and unencrypted despite containing credential material. | Encrypt and copy off-host; document a restore drill. |
| OPS-009 | ops | Graceful shutdown does not drain background metadata/preview tasks. | Track JoinHandles or mark runs failed on shutdown. |
| OPS-011 | ops | Nginx logs `Referer`, which can contain `?invite=CODE`. | Redact query or omit Referer. |
| OPS-012 | ops | `sampler2.pid` (a test-harness PID) is committed in `119c2ea`. | Remove it from the repo and add `*.pid` to `.gitignore`. |

## Merged / not carried forward

- `BUG-003` → merged into `BUG-200`.
- `BUG-004` → not a bug by itself: retaining tracks removed from Spotify is documented. The real defects are metadata/genre loss and position collisions, tracked as `BUG-302`.
- `BUG-005` → not reproducible; track locks serialize duplicate votes (1×200 + 1×409). Dropped.
- `BUG-006` → merged into `BUG-100`.
- `BUG-009` → merged into `BUG-115`.
- `BUG-012` → merged into `BUG-152`.
- `BUG-102`, `BUG-103`, `BUG-310` → merged into `SEC-101`, `SEC-102`, `BUG-100`.
- `BUG-255` → merged into `BUG-110`; `BUG-264` → merged into `BUG-113`; `BUG-309` → merged into `BUG-253`.
- `UX-011` → merged into `A11Y-002` (same opacity/contrast defect).
- `SEC-002`, `SEC-003`, `SEC-006`, `SEC-107` → merged into `SEC-101`, `SEC-102`, `SEC-111`, `BUG-100`.
- `OPS-006`, `OPS-010` → merged into `SEC-104`, `SEC-109`.
- Observer modal findings `UX-100`/`A11Y-100`/`A11Y-101` are obsolete: the track-detail modal was replaced by the inline `TrackDetailPanel`; the inline version is axe-clean and does not use a focus-trapping dialog.
- Observer `BUG-350`–`BUG-354` are merged into `BUG-150`, `BUG-152`, `BUG-110`, `BUG-100`, `BUG-302`.

## Accessibility (WCAG 2.2 AA, frozen build)

64 axe runs across ~62 page/state/viewport combinations, plus manual computation of ~6,075 text elements, keyboard walks, CDP accessibility trees, 320/720 reflow and touch-target scans. The three high issues are **A11Y-001/002/003** above (focus loss after actions, losing-side contrast, bottom nav obscuring focus). No critical axe violations; sealed `/standings` and `/stats` are axe-clean on desktop and mobile.

| ID | Severity | Issue | Fix |
|---|---|---|---|
| A11Y-004 | medium | Focus ring uses `--playlist-accent` (`#cd8e54`) at **2.40:1** on paper / 2.66:1 on card; default accent is 2.71/3.00:1. | Use `--playlist-accent-strong` or a fixed dark outline (4.89:1) with a paper halo. |
| A11Y-005 | medium | Input/search/select borders (`--color-line #d8cdb6`) are **1.37:1** on paper / 1.51:1 on card. | Use a ≥3:1 border token or inset ring. |
| A11Y-006 | medium | Small muted labels/placeholders miss 4.5:1: stats facet counts **2.74:1**, your-duels counts 2.74:1, home hints 4.20:1, “30 sec” 4.15:1, duel meta 4.47–4.50:1. | Remove `opacity-60`; use `--color-muted` (6.76:1); darken placeholders and listen-panel text. |
| A11Y-007 | medium | Standings/ranking row `aria-label` omits visible rank/W/D/score; axe `label-content-name-mismatch` on **50 rows** per run. | Drop the `aria-label`, add an `sr-only` Show/Hide affordance, or include all visible data. |
| A11Y-008 | medium | Auth mode switch has no `aria-selected`/`aria-pressed`; both mode and submit are named “Sign in”. | Use a real tablist or `aria-pressed`; name submit “Sign in to account”/“Create account”. |
| A11Y-009 | medium | Admin section nav has no active state (`aria-current`/`aria-pressed` all null). | Add `aria-current="page"`; announce/focus the new panel. |
| A11Y-010 | medium | Admin day-chart buttons are **19.3 × 144 px**; axe `target-size` on 14 nodes. | Add vertical padding/min-height ≥24 px and ≥5 px spacing. |
| A11Y-011 | medium | Scrollable admin “Recent picks” `ul` and table wrappers are not keyboard-focusable. | `tabindex="0"` + `aria-label` (or let the list grow). |
| A11Y-012 | medium | Signed-out `/duel`, `/your-duels`, `/your-ranking` have no `<main>` landmark; axe `landmark-one-main`/`region`. | Render the auth fallback inside `<main id="main">`. |
| A11Y-013 | medium | `/stats` heading order skips h2→h4 for story-card groups. | Change group headings to `<h3>`. |
| A11Y-014 | medium | Duel controls have duplicate/mismatched names (cover and Listen both “Play 30 second preview…”); pick name omits visible Side/year/genre; Undo omits “Z”. | Give distinct names; hide duplicate cover control from the AX tree; include visible metadata/key hints. |

**Low accessibility items:** A11Y-015 auth errors are announced but not tied to fields (`aria-invalid`/`aria-describedby`/focus); A11Y-016 zero-result filter state is not a live region; A11Y-017 search clear is 20×20; A11Y-018 skip link drops focus to `<body>`; A11Y-019 admin tables lack `caption`/`scope`; A11Y-020 reveal toast is visual-only; A11Y-021 multilingual title has no `lang`/`dir`; A11Y-022 loading skeleton has no heading/status/`aria-busy`; A11Y-023 design-page palette/glyph contrast.

## Visual / responsive (frozen build)

~220 screenshots across 7 viewports (320×640, 390×844, 820×1180, 1024×768, 1440×900, 1920×1080, 720×450@2) and 19 route/state groups, including all admin sections, sealed/revealed, empty/error/loading/paused, auth variants and the new inline `TrackDetailPanel`. No horizontal document overflow at any width; footer/bottom-tab collision was not found; empty, error and missing-art states lay out correctly.

| ID | Severity | Issue | Fix |
|---|---|---|---|
| UX-001 | medium | Admin nav active label becomes ink-on-ink while hovered/just clicked (`hover:text-ink` overrides `text-paper` on the ink pill). It is invisible in the sidebar and mobile strip. | Scope hover colour to `:not(.pv-admin-nav-item-active):hover` or add an active hover rule with paper text. |
| UX-002 | medium | Duel primary controls fall below the fold: at 1024×768 both pick buttons are at y=759–807 (below 768); at 1440×900 the cassette bottom and auto-preview toggle are clipped. `RecentDuels` is 306px at ≥1024 instead of capped. | Cap recent-duels height at `lg`, clamp cover/padding via `@media (max-height: 820px)`. |
| UX-005 | medium | Genre bar-chart labels truncate even on desktop (“ELECTRONIC/…”, “INDIE/ALTER…”), so several are ambiguous despite free space. | Widen the label column to ~7–9rem, allow two lines, or use short names + `title`. |
| UX-007 | medium | Mobile standings/ranking rows are ~120px because trailing W/D/score wraps to a full-width third row; only ~7 rows per screen on a ~6,000px list. | Keep score on the title line; shrink/hide the `D` label and reduce gaps below `sm`. |
| UX-012 | medium | Mobile duel has no visible pick affordance: desktop pick buttons are hidden; only the unmarked title/artist block votes. The prominent controls are preview-only. | Show a compact “Pick this one” chip/button per side, or make the whole card the pick target with an explicit chip. |

**Low visual items:** UX-003 320×640 recent-duels sliver (7px); UX-004 search field ~92px at 320 / clipped placeholder; UX-006 decade card half-empty and legend misaligned; UX-008 320px six-tab labels nearly collide; UX-009 mobile admin nav hides five sections with no scroll affordance; UX-010 danger-zone buttons wrap into tall ovals and the full-reset placeholder is clipped; UX-011 loser opacity (merged into `A11Y-002`); UX-013 reveal toast covers the title/actions for 2.6s; UX-014 long titles truncated with no full-title affordance (inline panel does not repeat the title).

## Performance (frozen build)

Lighthouse 13.6 (mobile simulated + desktop), Playwright CLS traces, 26-endpoint latency runs, 10–100-voter concurrency, `EXPLAIN (ANALYZE, BUFFERS)` at 150 and 2,000 tracks, memory/CPU sampling and dump/restore timing. **PERF-001** (deadlocks/500s) and **PERF-003** (CLS 0.65–1.60) are high and detailed above.

| ID | Severity | Issue | Fix |
|---|---|---|---|
| PERF-002 | medium | Timezone validation costs **32–38 ms** on every matchup/summary (`37.7 ms` vs `5.7 ms` without tz; `pg_timezone_names` scan 35–79 ms). | Validate IANA names once into a bounded set/cache; fall back to UTC. |
| PERF-004 | medium | Fonts are **185 kB / 189 kB transfer = 58–59 %** of page weight (5 files; only 2 preloaded); mobile LCP 2.98–3.74 s. | Subset/reduce weights, one mono weight, preload only above-the-fold fonts, skip gzip for `.woff2`. |
| PERF-005 | medium | Main JS **345.6 kB raw / 107 kB gzip**; new `TrackDetailPanel` is duplicated into Chart/Ranking chunks; mobile TBT 99–354 ms. | Vendor `manualChunks`, shared/lazy panel chunk, defer admin StoryCards. |
| PERF-006 | medium | 16-connection pool saturates at 100 voters: matchup p50 **526 ms / p95 753 ms**, 11.7 votes/s, DB CPU only 14–22 %. | Raise pool to 32, remove tz round-trip, combine `user_progress` counters. |
| PERF-007 | medium | Personal ranking replay runs on every ranking/summary/detail request; 1,500 votes → 45.8 ms p50 / 67.6 p95; 25 concurrent → 317 ms p95. | Cache replay per (user, playlist), invalidate on vote/undo; or persist per-user Elo. |
| PERF-008 | medium | Shared 3-permit `heavy_work` semaphore serialises admin insights/stories and starves public reads (insights p95 904 ms at 2 k; cached chart up to 1.48 s). | Separate admin/public semaphores; cache insights 5–10 s; compute aggregates once. |
| PERF-009 | medium | `/api/me/duels` default page returns **106 kB** (50 duels × two full tracks + inline art). | Return compact rows `{id,title,artists,image,winner,seen_at}`; lazy-load art. |

**Low performance items:** PERF-010 inline album art adds ~29 kB to a 52.6 kB chart page; PERF-011 `ILIKE '%q%'` + double ranking scan is fine at 2 k but O(n); PERF-012 no `seen_pairs(playlist_id)` index; PERF-013 admin users runs two LATERAL aggregates per user and CSV is unbounded; PERF-014 cache entries are ~52 kB, clear-all eviction, no ETag; PERF-015 no brotli and `.woff2` is gzipped; PERF-016 admin ranking is uncached; PERF-017 15 s status polling + 5 s auth cache; PERF-018 CSV builds full strings; PERF-019 duel first load makes two third-party preview requests; PERF-020 track drawer does ~10 queries + full replay per open.

**Scale result:** at 2,000 tracks / 4,705 votes / 4,883 seen pairs / 111 users, DB is 17 MB; dump/restore 0.59 s/1.01 s; chart/ranking/insights SQL remain fast (chart page 10.2 ms, ranking 45.8 ms, insights 49.6 ms, admin stories 88–95 ms). The bottlenecks are the 16-connection pool, per-request replay and the 3-permit semaphore, not the schema.

## Admin E2E

The frozen recheck covered all nine admin sections on `119c2ea` with stack 6 rebuilt and restored to baseline. Overview counts matched `/api/admin/insights` and `/api/admin/stats` for 1/7/14/30/90 days; voter pagination/sorts/search, rename, password reset, disable/enable, force logout, clear picks and delete were exercised with correct server-side effects; reveal/schedule/pause and all destructive confirmations behaved correctly; full reset kept the acting admin and Spotify connection; no page-level horizontal overflow at 390px.

The main admin risks were failure-state honesty and feedback: section 500s rendered as empty data with a stale “Loading…” label (`BUG-250`), destructive success notices were lost on remount (`BUG-251`), preview backfill never reported completion (`BUG-252`), the playlist panel hid 144 of 150 tracks (`BUG-253`) and expired invites could not be revoked (`BUG-254`). Spotify start/schedule failures used the generic 500 from `BUG-110`. All six are now fixed; the remaining admin findings are the low-priority items in the table above (`BUG-256`–`BUG-265`).

## What worked / verified clean

- Elo maths matches `backend/src/elo.rs` exactly; zero-sum deltas, K decay, initial 1000, wins/comparisons invariants.
- Streaks match an independent `zoneinfo` implementation across Amsterdam/New York/Sydney/UTC/DST boundaries, goals 1/3/100 and undo-of-today.
- Reveal/seal gating is synchronous: public chart/artists/albums/stories blocked while hidden; averages nulled; admin ranking available; cache cleared.
- Authz: all admin routes 401 anonymous / 403 voter; no IDOR on votes, undo, history or ranking; mass assignment blocked; last-admin guards; session revocation via admin actions works.
- JWT forgery (`alg:none`, tampered, expired, forged `is_admin`) rejected; cookie flags correct; CSRF not exploitable with JSON-only + SameSite=Lax + no CORS.
- SQL injection, stored/reflected XSS, path traversal, SSRF and sealed-standings leakage not found; no secrets in the built JS.
- API contract and `types.ts` match for all tested endpoints, including the new track-dossier response.
- Voting validation matrix (same track, winner outside pair, unserved/other-user/inactive/double vote) returns 400/409 correctly; pause/schedule block writes but not reads.
- Admin clear-votes/delete-user rebuild is an exact replay when replay order is well-defined; invite/auth boundaries are solid.
- Frontend builds cleanly; 35/35 backend tests; migrations apply fresh; source and built assets were in sync at the snapshot.
- E2E: invite links, auth persistence, duel keys/swipes, counters, history filters, ranking filters, sealed/revealed standings, stats, mobile layout and inline detail expansion worked.
- Spotify mock: pagination/dedupe/skips, metadata/preview backfill, OAuth state, refresh rotation, scheduling metadata-only, force start, error mapping and pause all worked.
- Accessibility: no critical axe violations; sealed `/standings` and `/stats` are axe-clean on desktop/mobile; reduced motion honoured; no horizontal reflow at 320/720; no duplicate ids; inline `TrackDetailPanel` has correct `aria-expanded` and no modal focus trap.
- Performance: reveal invalidates cache immediately; compressed JSON is 10–16× smaller; cold-cache stampede is coalesced; caches/rate limiter are bounded; backend RSS stays 106–109 MB; hot-path indexes are used; backup/restore is fast (17 MB DB in ~1.6 s).

## Gaps / untested

- **Real Spotify** was unreachable from the sandbox (container DNS), so import/OAuth/preview/CDN behavior is mock-only; real Feb-2026 item migration, consent, rate limits and embed markup remain unverified.
- **Chromium only**; no Firefox/WebKit/Safari or physical devices; safe-area insets emulated only via CSS fallback.
- **No live nginx/TLS/certbot**; HSTS/TLS findings are static review.
- Rate-limit 429 was proven by restarting a stack with low limits, not with production defaults.
- 2,000-track scale measurements are from the performance lane; full 2,000-track end-to-end journeys were not run.
- Admin export/import success against real Spotify was not possible; mock only.
- No CI workflow exists; formatting/clippy gates are manual.
- The track-detail feature changed during the run from a modal to an inline panel; only the inline version is in scope.
- `sampler2.pid` is committed in `119c2ea`; it is a test-harness PID file, not product code, and should be removed.
- Accessibility: no NVDA/JAWS/VoiceOver session; CDP accessibility tree only. Contrast was computed over the paper base (gradients darken by ≤~1%); actual audio was not audible in headless Chromium.
- Performance: the host was shared (load average 1.7–30), so TBT/Speed Index and some tails are noisy; INP was not measured directly; frontend 2,000-track Lighthouse numbers are superseded by the final-build baseline.
- Visual: 200% zoom was emulated with a 720×450 CSS viewport at DPR 2; admin sections were sampled at the top and long sub-panels were not fully scrolled.

## Suggested release checklist

1. ✅ **Done** — correctness/security blockers: `SEC-001`, `SEC-100`, `BUG-150`.
2. ✅ **Done** — reliability/accessibility blockers: `PERF-001`, `PERF-003`, `A11Y-001`, `A11Y-002`, `A11Y-003`.
3. ✅ **Done** — `SEC-102`, `BUG-101`, `BUG-152`, `SEC-101`, `BUG-100`, `BUG-151`.
4. ✅ **Done** — `BUG-200`, `BUG-201`, `BUG-250`, `BUG-110`, `BUG-300`, `BUG-301`, `BUG-302`, `BUG-305`.
5. ✅ **Done** — `A11Y-004`–`A11Y-007`. Remaining accessibility: `A11Y-008`–`A11Y-014`.
6. ✅ **Done** — `UX-001`, `UX-002`, `UX-012`. Remaining visual: `UX-005`, `UX-007`.
7. ✅ **Done** — `cargo clippy --all-targets -- -D warnings` exits 0; add it to CI. `cargo fmt --check` remains the low `BUG-008`.
8. ✅ **Done** — healthcheck, stop grace, DB URL override, atomic backups, restore maintenance, scheduler timeout, timezone cache. Remaining ops: HSTS/TLS, container hardening.
9. Rebuild from the fixed tree and re-run the high/medium smoke matrix plus the release checklist before deploy.
