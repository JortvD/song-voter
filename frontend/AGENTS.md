# Frontend — AGENTS.md

Supplement to `../AGENTS.md`. Read the root file first for hard boundaries,
commands and verification expectations. This file covers the React app in
`frontend/`.

## Stack and layout

React 19, TypeScript 5.9 in strict mode, Vite 7, Tailwind CSS 4, React Router 7.
No state-management, component-library or CSS-in-JS dependency. Keep the stack
small; prefer existing primitives and native browser APIs over new packages.

| Path | Responsibility |
|---|---|
| `src/main.tsx` | mounts `App` inside the providers |
| `src/App.tsx` | routes, lazy page imports, shell (header/footer/tabs), error banner |
| `src/pages/` | route-level screens: Home, Vote (duel), history, ranking, standings, stats, admin, design |
| `src/components/` | shared UI, feature panels and admin sections |
| `src/components/duel/` | cassette card, sides, swipe hook, preview controls, recent duels |
| `src/components/admin/` | mission-control panels and tables |
| `src/components/streak/` | streak bar/card/dots/flame |
| `src/context/` | `AuthContext` and `PlaylistContext` providers |
| `src/lib/api.ts` | the **only** place that calls `fetch`; typed API methods |
| `src/lib/types.ts` | response/domain types mirroring backend serde JSON |
| `src/lib/sound.ts` | WebAudio action sounds, respects the Sound toggle |
| `src/lib/playlistAccent.ts` | extracts and applies the playlist accent colour |
| `src/lib/time.ts` | small time helpers |
| `src/index.css` | Tailwind import, fonts, theme tokens, base and component classes |

## Commands

```bash
cd frontend
npm ci
npm run dev     # Vite dev server on :5173
npm run build   # tsc --noEmit && vite build — required gate
npm run preview # preview the production build
```

There is no ESLint/test script. `npm run build` is the type and build gate.
TypeScript runs with `strict`, `noUnusedLocals`, `noUnusedParameters`,
`noFallthroughCasesInSwitch` and `verbatimModuleSyntax`, so:

- use `import type { … }` for type-only imports;
- remove unused variables, imports and parameters (prefix truly unused
  parameters with `_` only when the signature requires them);
- do not use CommonJS or require syntax.

If npm is unavailable, build with `docker build --target frontend .` from the
repo root.

## API client rules

- All HTTP goes through `src/lib/api.ts`. Do not call `fetch` in components,
  contexts or pages.
- Add one method per endpoint to the `api` object, typed with a type from
  `src/lib/types.ts`. Keep request/response field names in snake_case exactly as
  the backend serializes them.
- `request()` uses `credentials: 'same-origin'`, JSON headers only when there
  is a body, a 15s default timeout and a 200s timeout for long playlist
  operations. Pass `timeoutMs: LONG_RUNNING_TIMEOUT_MS` for imports/backfills.
- Errors are `ApiError` with `message` and `status`. Show `error.message` to
  users; do not parse response bodies again.
- A 401 on a non-auth route dispatches `ull:unauthorized`; `AuthProvider`
  listens and clears the user. Do not add per-component session handling.
- Lists and searches that can race should accept an `AbortSignal` and pass it
  to `request`; check `signal.aborted` before setting state.
- Add `tz` with `withTimezone(path, timezone)` for endpoints whose streaks,
  heatmaps or daily stats depend on the voter's local day.

## Routing and guards

- Routes are declared in `src/App.tsx`. `HomePage` and `VotePage` are eager;
  admin, standings, stats, ranking, history and design pages are lazy. Keep new
  heavy screens lazy.
- Legacy paths redirect: `/vote` → `/duel`, `/ranking` → `/your-ranking`,
  `/chart` → `/standings`; unknown paths go home.
- Use `RequireAuth` / `RequireAdmin` rather than checking `user` inline in a
  page.
- `DocumentTitle` derives the tab title from the route plus the active playlist.
  Add new screens to `SCREEN_TITLES`.
- Admin sections are URL-backed via `?section=…` (`AdminPage.tsx`). Deep links
  such as the Spotify OAuth return (`?spotify=…`) must keep working; preserve
  unrelated search params when changing tabs.

## State and data flow

- `AuthProvider` owns the current user and login/register/logout actions.
- `PlaylistProvider` owns the active playlist/reveal status, refreshes on route
  changes and every 15s, and applies the extracted playlist accent. Use
  `usePlaylist()` instead of fetching `/api/chart/status` in a page.
- Local component state + `useCallback`/`useMemo` is the norm. Do not introduce
  a global store for one screen.
- Keep effects cancellation-safe (AbortController or an `active` flag). This
  codebase has had post-unmount state updates; do not add new ones.
- Mutations that change shared data (votes, reveal, playlist, settings, user
  admin actions) should refresh the relevant local data and/or call
  `refreshPlaylist()` so the shell stays consistent.

## Styling and visual language

The design is a warm, printed cassette/tape aesthetic: cream paper, ink-blue
text, cobalt and safety-orange accents, offset hard shadows, 2px borders, mono
uppercase micro-labels and Bricolage/DM Sans typography. The `/design` route is
the style tile; check it before inventing new visual patterns.

- Use tokens from `@theme` in `src/index.css`: `paper`, `card`, `ink`, `muted`,
  `line`, `accent`, `cobalt`, `butter`, `ok`, `danger`, plus the `font-sans`,
  `font-display`, `font-serif` and `font-mono` families. Do not hard-code new
  hex colours that duplicate a token.
- Reuse component classes where they fit: `.panel`, `.pv-card`, `.pv-card-soft`,
  `.pv-label`/`.label`, `.btn`, `.btn-ghost`, `.field`, `.pv-sticker*`,
  `.pv-link`, `.pv-admin-*`.
- Keep the playful details that define the product: rotated stickers, hard
  offset shadows, tape reels, stamp/tilt feedback, playlist accent. Do not
  replace them with generic dashboard styling.
- Icons are inline SVG or text glyphs; there is no icon library.
- Use `min-h-svh`/`100svh` (not `100vh`) for mobile-stable layout. The app has a
  fixed header and bottom tabs; respect the existing scroll padding so focused
  controls are not hidden.
- Accessibility is part of the design: semantic buttons, `aria-*` on toggles,
  visible focus, `sr-only` labels, `role="alert"` for errors and
  `role="status"` for notices. Keyboard shortcuts (`1`/`A`/`←`, `2`/`B`/`→`,
  `Z`, `Space`) must not fire while an input/textarea/select is focused or when a
  modifier is held. `Space` must not pick a track.

## Copy and terminology

- UI copy is English. Use product terms: **duel**, **pick**, **Your Ranking**,
  **The Standings**, **The Reveal**, **The Stats**, **voter**, **season**.
- Never show a score, rating, win count or comparison count before The Reveal
  except inside admin screens and Your Ranking (personal picks).
- Keep the voice dry, warm and concise. Error messages should say what failed
  and what the user can do; do not expose raw backend internals.
- Dates/times shown to voters use their local timezone; backend UTC timestamps
  are ISO strings. Use the existing helpers and `Intl` formatting rather than
  hand-rolled parsing.
- Sound is opt-in via the footer Sound toggle and `lib/sound.ts`; new sounds
  should go through that module so the preference is respected.

## Adding an API-backed screen or panel — checklist

1. Add/adjust the response types in `src/lib/types.ts` to match the backend.
2. Add the method to `src/lib/api.ts`, including `signal` and long-timeout
   options where appropriate.
3. Build the page in `src/pages/` or a focused component; keep data loading in
   the component/hook, not in `App.tsx`.
4. Handle loading, empty, error and unauthenticated states. Show `ApiError`
   messages in a visible `role="alert"` region.
5. Add auth/admin guards at the route or panel boundary.
6. Use existing layout, tokens and component classes; verify mobile and
   desktop sizes and keyboard focus.
7. Update `README.md` if the feature is user-visible and documented there.
8. Run `npm run build` and fix all type errors and unused-symbol failures.

## Pitfalls specific to this app

- `frontend/dist/` is generated and served by the backend in production. Never
  edit or commit it.
- The Vite dev server proxies only `/api`, `/healthz` and `/readyz`; use it
  rather than hard-coding backend origins. `VITE_API_PROXY` can point at the
  compose host port (e.g. `http://127.0.0.1:8765`).
- The SPA is served with `no-cache`; hashed assets/fonts are immutable. Do not
  add query-string asset hacks.
- Spotify preview playback is native `Audio`, not the embed iframe. Only one
  side plays at a time; changing duel resets both; a track without
  `preview_url` shows a Spotify link instead.
- The tight mobile layout (recent-duels height, auto-preview placement, fixed
  status strip) is deliberate. Test scroll and browser-chrome changes before
  altering heights.
- `noUnusedLocals` makes unused destructured values a build error; use only what
  you need or prefix with `_` where the API requires the slot.
- Type assertions are acceptable at API boundaries, but prefer discriminated
  unions and narrow types inside components. Do not silence errors with `any`.
