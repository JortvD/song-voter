# Playlist Voter — Retention & Delight Playbook

**Context:** invite-only, cassette-themed duel voting for a club / class / team of roughly 15–50 people. The next batch should optimize for **daily habit and retention**, mostly with **frontend quick wins**, while keeping the **analog ritual** intact.

**Thesis:** Playlist Voter already has a strong *session* (a duel) and a strong *event* (The Reveal). What it lacks is a **daily reason to open the app**, a **safety net when a session is interrupted**, and a **social surface that makes the room feel alive**. Fix those before adding more game mechanics.

---

## Where the loop leaks today

1. **Opening `/duel` immediately records a seen pair.** Refresh, back button, or an accidental tap creates a "skipped" duel, and there is no way to resume it. Curiosity is silently punished, and the skipped count inflates.
2. **The streak is invisible where people land.** `/api/me/summary` already returns `streak`, but the home page only uses picks, songs ranked, and the personal #1. The daily habit exists in the data, not in the UX.
3. **There is no return trigger outside the app.** No calendar entry, no installable app, no reminder. The streak depends entirely on memory.
4. **The Reveal is a page, not an event.** No projector mode, no calendar reminder, no prediction game, no shareable artifact. The biggest emotional payoff is under-produced.
5. **Private rankings are isolated.** There is no "you vs. the room", no taste twin, no post-pick surprise. Voting feels solo even when 30 people are doing it.
6. **Seasons have no afterlife.** A full reset deletes everything. For a club, legacy is the long-term retention feature.
7. **The standings API supports search and filters, but the standings UI does not expose them.** After The Reveal, finding a song means scrolling.
8. **Invites are codes, not links.** The admin has to copy a code and send it separately; onboarding friction is higher than it needs to be.

---

## Design principles

- **Analog ritual, not casino.** No coins, loot boxes, loss-streak pressure, or public shaming. Rewards should feel like stickers, stamps, liner notes, and ticket stubs.
- **Protect the seal.** Nothing may reveal the group standings before The Reveal. Post-pick aggregates are acceptable only after the pick, and only if they cannot be used to reconstruct the ranking.
- **One clear next action per screen.** The duel screen should always answer "what do I do now?" in under a second.
- **Quick wins first.** Prefer ideas that use existing endpoints. Add schema only when the idea has earned it.
- **Measure behavior, not clicks.** For a club of 15–50, don't run underpowered A/B tests. Run two-week sequential iterations, watch the numbers, and ask three questions in the group chat.

---

## Summary: the shortlist

**If you only build five things:**

1. **B1 + B2 — Resume the current duel + rescue skipped duels.** The highest-leverage friction fix, and it is frontend-only with the existing API.
2. **A1 + A2 — Put the streak on the home page, with an at-risk nudge and a goal celebration.** The habit already exists; make it visible.
3. **C1 + C2 — Countdown to The Reveal everywhere, plus an add-to-calendar button.** Make the event impossible to miss.
4. **C3 — Reveal theater / party mode.** A projector-friendly ceremony for the club night.
5. **E1 — Shareable cassette J-card.** Your #1, your top five, your streak; made to be posted in the group chat.

**Then:** F1 (standings search/filter), E4 (invite links + QR), D1 + D2 (taste twins + post-pick room split), A8 (Today's Cut), F8 (onboarding).

**Later, if the club keeps running seasons:** E9 + E10 (archive + hall of fame), C7 (prediction league), F15 (web push), D7 (fantasy top five).

---

## Idea catalogue

Effort: **S** = frontend-only, roughly a day. **M** = frontend plus a small endpoint or a substantial component, 2–4 days. **L** = schema, scheduler, or a new system, 1–2 weeks.

### A. Daily ritual & streaks

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| A1 | **Home streak card + at-risk nudge** — show current streak, today's `x / goal`, last seven days, and "2 picks left today" after the local evening. | The data is already in `/api/me/summary`; the home page is where people arrive. | S | Goal-met days, home → duel CTR |
| A2 | **Goal-met celebration + milestone stamps** — stamp animation at the goal, and small 3/7/14/30-day badges. | Turns a quiet threshold into a moment of completion. | S | Next-day return after milestone |
| A3 | **Full-month streak calendar** — a 30-day heat strip instead of only the last seven days. | Makes consistency visible and gives a reason to protect the chain. | M | Goal-met days |
| A4 | **Weekly recap card** — "Your week: 42 picks, 5-day streak, top song, most-picked decade." | Gives a lapsed voter a reason to open the app even when they don't want a full session. | S | Weekly return rate |
| A5 | **Sticky mobile "today" bar** — above the bottom tabs when the streak is at risk or skipped duels exist. | Puts the next action in the thumb zone. | S | CTA click, goal-met days |
| A6 | **Streak freeze / tape repair** — one missed day can be repaired by a pick, once per season. | Prevents the "I broke it, why come back?" churn moment. | L | Return after a missed day |
| A7 | **PWA install + app badge** — installable app shell and an optional badge for remaining daily picks. | A home-screen icon is a daily trigger that costs nothing. | M | Installs, D7 |
| A8 | **Today's Cut** — one deterministic track per day on the home page, with a 30-second preview and a "Duel it" CTA. | A tiny daily listening ritual for people who don't want a full session. | M | Home → duel CTR |
| A9 | **Last-chance banner** — when the reveal or pause is within 24 hours, show a countdown and "finish your picks". | Converts procrastination into a final session. | S | Picks in the last 24h |

### B. Friction & recovery

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| B1 | **Resume current duel after refresh** — keep the served pair in `sessionStorage`; restore it instead of requesting a new one. | Stops a refresh from silently creating a skipped duel. | S | Duel completion rate |
| B2 | **Skipped-duel rescue mode** — `/api/me/duels?status=skipped` already returns the pairs; walk through them and vote with the existing `/api/vote`. | Recovers abandoned sessions and increases coverage. This is the single best quick win. | S | Skipped → picked conversion |
| B3 | **Explicit Skip / Too close to call** — a visible skip button and `S` shortcut; fetch the next duel without pretending the user vanished. | Removes guilt and keeps the session moving. | S | Picks per session; skipped rate |
| B4 | **Better exhausted state** — when no unseen pairs remain, offer rescue, your top ten, share, and next-season teaser. | The current dead end is a churn point. | S | Return after exhaustion |
| B5 | **Offline / error recovery** — offline banner, retry-next-duel, and a "your pick is saved" state. | Club nights happen on bad Wi-Fi. | S | Error recovery rate |
| B6 | **Finish-your-tape checklist** — "12 skipped duels · 3 picks to goal · 47% of the tape heard." | Turns a vague sense of unfinished business into a concrete plan. | S | Coverage, goal-met |
| B7 | **Preview fallback** — if a preview is missing, show a clear Spotify link and let the other side play. | Avoids a dead end on tracks with no preview. | S | Preview play rate |

### C. The Reveal & anticipation

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| C1 | **Reveal countdown everywhere** — a compact "seal opens in 3d 4h" chip in the header and on standings/stats, not just home. | Keeps the event top of mind on every visit. | S | Reveal-day attendance |
| C2 | **Add to calendar (.ics)** — one click for the season start and The Reveal, generated client-side. | The simplest reliable reminder that does not need push infrastructure. | S | Calendar adds, attendance |
| C3 | **Reveal theater / party mode** — full-screen, projector-friendly reveal of #1–#10 with big art, pacing controls, optional sound and confetti. | Turns The Reveal into a club night and a story people retell. | M | Reveal session length, shares |
| C4 | **Printable / shareable reveal poster** — a generated top-ten poster in the cassette visual language. | Gives the organizer something to post without design work. | S | Shares, next-season invites |
| C5 | **"The B-Side" / Wooden Spoon** — after the reveal, show the bottom ten and playful awards for the songs nobody defended. | The losers are often funnier than the winners. | S | Post-reveal time on page |
| C6 | **Reveal quiz / guess the score** — "guess the #1 score" or "pick the top three" as a local side game. | A low-stakes game for the room while the reveal plays. | S | Participation |
| C7 | **Prediction card / sealed guesses** — before the reveal, predict group #1, last place, most divisive album; score the predictions afterward. | A season-long side quest and a reason to return on reveal day. | L | Prediction participation, D1 after reveal |
| C8 | **Blind bracket after the reveal** — a local knockout among the top eight; "people's champion" vs official #1. | Reuses data already on the standings page; no ranking changes. | S | Post-reveal engagement |
| C9 | **Voter superlatives** — "most contrarian", "taste twin", "night shift", "completionist", "longest streak". | Recognition is the cheapest social reward; it fits the liner-notes voice. | M | Profile visits, shares |
| C10 | **Compare with the room** — after the reveal, show your rank vs the group, your biggest disagreement, and your most contrarian pick. | Turns a private list into a conversation starter. | S | Post-reveal engagement |

### D. Social fabric

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| D1 | **Taste twins & nemesis** — compute agreement from overlapping votes; show "you agree most with @maya" and "your nemesis is @sam". | The club already knows each other; this gives them a new thing to tease each other about. | M | Sessions per voter, 7-day return |
| D2 | **Post-pick room split** — after you pick, show "58% of the room picked the other side" for that duel. | A small surprise after every pick; conversation fuel without leaking the standings. | M | Picks per session, shares |
| D3 | **Live room pulse** — "12 voters active today · 342 picks this week · longest streak 9 days" plus anonymized recent activity. | Makes the room feel occupied even when you are voting alone. | M | Home engagement |
| D4 | **Sticker sheet / tape case** — collectible stamps for milestones: first pick, 10/50/100 picks, 7-day streak, decade explorer, night owl, completionist. | Analog collection mechanics that fit the cassette world; no coins or loot boxes. | M | Profile visits, return |
| D5 | **Reactions** — after a pick, leave a one-tap reaction (🔥, 😭, 💎, 🤡) and see aggregate counts. | Lightweight banter without the moderation burden of comments. | M | Reactions per pick |
| D6 | **Profile / "Your Tape" hub** — one page for streak, badges, taste profile, top five, history, and share card. | Gives the personal side of the product a home and a reason to return. | S | Profile visits |
| D7 | **Fantasy top five** — before the reveal, each voter drafts five songs; score them against the final standings. | A season-long competition that rewards engagement without touching the official ranking. | L | Participation, cross-season return |
| D8 | **Teams / crews** — split voters into two or three teams, score by average picks or average track rating. | A club-sized competition layer for special seasons. | L | Team participation |

### E. Identity, sharing & invites

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| E1 | **Shareable cassette J-card** — generate a downloadable/shareable card with your #1, top five, streak, and a cassette spine. | Identity plus easy group-chat sharing. Use text/color-only art if album-art CORS is a problem. | S | Shares, invite conversion |
| E2 | **Season wrap** — a Wrapped-style recap of your genres, decades, top song, agreement, streak, and picks. | A natural end-of-season share moment and a bridge to the next season. | S–M | Shares, next-season return |
| E3 | **Personal liner notes / taste profile** — auto-generated taste summary from your ranking: favorite decades, genres, artist loyalty, contrarian score. | Makes the private ranking feel like a possession, not a table. | S | Profile visits, shares |
| E4 | **Invite links + QR + voter kit** — `?invite=CODE` prefill, a copy-ready message with URL, code, and start time, and a QR code for the group chat. | Removes the two-step invite friction and makes the admin's life easier. | S | Invite redemption, time-to-first-pick |
| E5 | **Deep links to songs** — every standings/ranking row gets a copy link and a highlighted anchor on load. | Makes "look at this" easy in the group chat. | S | Shares, referrals |
| E6 | **Copy standings as text** — a plain-text top ten for WhatsApp/Discord. | The organizer's most common sharing job, done in one click. | S | Shares |
| E7 | **Season ticket stub** — a small memento with the playlist, dates, your rank, and your top song. | A physical-feeling artifact for a club season. | S | Shares |
| E8 | **Tape shelf / archive** — past seasons as cassettes with final standings and your rank. | Legacy is the strongest cross-season retention feature. | L | Cross-season return |
| E9 | **Hall of fame / all-time records** — best song, biggest upset, longest streak, most agreement, most seasons. | Gives the club a shared history and a reason to keep playing. | L | Cross-season return |

### F. Quality of life, accessibility & admin

| ID | Idea | Why it hooks | Effort | Signal |
|---|---|---|---|---|
| F1 | **Standings search + filters** — search box and artist/decade/genre filters; the API already supports `q`, `artist`, `decade`, `genre`. | The biggest obvious UX gap after The Reveal. | S | Search usage, time on standings |
| F2 | **Your Ranking search + preview** — search, artist filter, and tap-to-preview like the duel history. | Makes a long personal list usable. | S | Ranking engagement |
| F3 | **History search/filter** — find a duel by song, artist, or result; client-side first, server-side later. | "Where was that duel?" is a real support burden. | S–M | History usage |
| F4 | **Keyboard shortcuts overlay** — a small `?` cheat sheet for pick, undo, preview, skip. | The shortcuts exist; make them discoverable. | S | Shortcut usage |
| F5 | **Reduced-motion / accessibility pass** — respect `prefers-reduced-motion`, improve focus order and labels, test keyboard-only. | Inclusive by default; also improves the projector experience. | S | Audit checklist |
| F6 | **Night-session mode** — dim the UI and enlarge the art for listening in a dark room. | A cozy ritual that fits the cassette identity. | S | Mode usage |
| F7 | **Crossfade / volume controls for previews** — smooth A/B switching and a volume slider. | Small polish that makes the core interaction feel expensive. | S | Preview play rate |
| F8 | **Onboarding tour / practice duel** — a three-step first-run overlay and a no-pressure sample. | The first session decides whether there is a second. | S | Time-to-first-pick, first-session completion |
| F9 | **Admin preflight checklist** — Spotify connected, playlist imported, preview coverage, start time, reveal scheduled, invite link copied. | Prevents the admin mistakes that kill a season before it starts. | S | Successful starts |
| F10 | **Admin season recap export** — a ready-to-post text summary and top-ten image. | Makes it easy to promote the next season in the group chat. | S | Shares, next-season invites |
| F11 | **Bulk invites / QR** — generate ten codes at once, copy all, or show one QR. | Club organizers invite in batches. | S | Invite redemption |
| F12 | **Admin retention view** — streak distribution, skipped-duel rate, goal-met rate, first-week funnel. | Makes the next iteration evidence-led instead of vibes-led. | M | Measurement quality |
| F13 | **Discord / WhatsApp digest** — a nightly or weekly webhook with picks, streak leaders, and reveal countdown. | For a club, the group chat is the real notification channel. | M | Re-engagement |
| F14 | **Web push reminders** — a daily "2 picks left" and a reveal reminder. | The strongest retention lever, but it needs backend, VAPID, subscriptions, and permission UX. | L | D1/D7, goal-met days |

---

## Recommended iteration ladder

Because the audience is 15–50 people, treat these as **sequential two-week iterations**, not randomized experiments. Use a before/after lookback and a short qualitative check-in.

| Iteration | Theme | Ship | Hypothesis | Primary metric | Guardrail |
|---|---|---|---|---|---|
| 0 | **Instrument** | Baseline from `/api/admin/insights?days=30`, plus a tiny `client_events` endpoint if you want click-level data. | You cannot improve what you do not baseline. | Completion rate, goal-met rate, 7-day return | No PII, no third-party analytics |
| 1 | **Don't lose the session** | B1, B2, B3, B4, B5, B6, F8 | Most skipped duels are accidental; resume + rescue will lift picks per session and coverage. | Picks / duels served; skipped → picked conversion | Picks per active voter should not fall |
| 2 | **Give today a shape** | A1, A2, A4, A5, A9, A8 | A visible streak, a daily cut, and a last-chance nudge will turn "maybe later" into "just two picks". | % of active voters meeting the goal 3+ days/week; D1/D7 | Skip rate should not rise |
| 3 | **Make The Reveal a night** | C1, C2, C3, C4, C5, C8, C10, E1, E2 | A shared event plus a shareable artifact will increase reveal attendance and next-season invites. | Unique voters on reveal day; shares; invite redemptions | No pre-reveal leaks |
| 4 | **Make the room feel alive** | D1, D2, D3, D4, D5, D6, C9 | Social comparison and recognition will increase sessions per voter and week-over-week return. | Sessions / voter / week; profile visits; 7-day return | Aggregate-only; no harassment surface |
| 5 | **Make it last** | E8, E9, C7, D7, F13, F14, A6 | Legacy and reminders extend retention across seasons. | Prior-season voters returning; streak repair usage | Complexity and privacy budget |

---

## Experiment cards for the top five

### 1. Resume + rescue

- **Change:** restore the served pair from `sessionStorage` on mount; add "Rescue skipped" on home, duel, and history; walk skipped pairs through the existing vote endpoint.
- **Hypothesis:** a meaningful share of "skipped" duels are accidental; recovery lifts completion and coverage without changing the ranking.
- **Measure:** picks / duels served (target +10–15% relative), skipped → picked conversion (target 25%+), coverage.
- **Guardrail:** picks per active voter and session length should not drop; no duplicate votes.
- **Decision:** keep if completion rises for two consecutive weeks; otherwise simplify the recovery entry point.

### 2. Home streak card + at-risk nudge

- **Change:** add a compact streak card to `HomePage` using the `streak` already returned by `/api/me/summary` (pass `localTimezone()` so the day boundary is correct); show "today done" or "N picks left"; add a goal celebration.
- **Hypothesis:** making the streak visible at the landing page increases goal completion and next-day return.
- **Measure:** % of active voters meeting the goal 3+ days/week; D1/D7; home → duel CTR.
- **Guardrail:** total picks per voter should not spike then crash; watch for streak-anxiety feedback.
- **Decision:** keep if goal-met rate rises without a drop in week-two picks.

### 3. Reveal countdown + calendar

- **Change:** a global "seal opens in…" chip on home, standings, and stats; an `.ics` download for the start and reveal.
- **Hypothesis:** repeated exposure and a calendar entry increase reveal-day attendance.
- **Measure:** unique voters on reveal day and the 24 hours after; calendar downloads; share clicks.
- **Guardrail:** no change to the sealed state or any pre-reveal data exposure.
- **Decision:** keep if reveal attendance rises; if not, test a direct reminder channel (Discord or push).

### 4. Reveal theater

- **Change:** a projector-friendly route that reveals top ten one at a time, with big art, pacing controls, optional sound, and a final "share the poster" CTA.
- **Hypothesis:** a produced reveal is more memorable and drives next-season conversation.
- **Measure:** reveal session length, number of voters present, shares, next-season invite redemptions.
- **Guardrail:** the normal standings page must remain unchanged and accessible.
- **Decision:** keep if the club uses it for the next reveal; interview three attendees.

### 5. Shareable J-card

- **Change:** generate a cassette J-card from your top five and streak; use the Web Share API, a download fallback, and a copy-as-text fallback.
- **Hypothesis:** an easy artifact increases group-chat sharing and invite conversion.
- **Measure:** share clicks, invite redemptions, new registrations from shared links.
- **Guardrail:** no private data beyond what the user chooses; text/color-only art avoids album-art CORS issues.
- **Decision:** keep if shares lead to at least one new voter or a measurable bump in invite use.

---

## Measurement plan

**North star:** weekly active voters who cast at least one pick on **3+ days**.

**Supporting metrics:**

- picks per active voter per week
- duel completion rate (`picks / duels served`)
- skipped-duel recovery rate
- % of active voters meeting the daily goal 3+ days/week
- first-session completion and time-to-first-pick
- reveal-day attendance and post-reveal return
- invite redemption rate
- shares, calendar adds, installs

**Baseline without new instrumentation:** `/api/admin/insights?days=30` already gives picks, voters, active voters, picks per voter, coverage, and daily activity. Add a small `client_events` table only for the interactions that matter (rescue, calendar, share, theater, search, install). Keep it first-party, rate-limited, and free of PII.

**Qualitative loop:** after each iteration, ask the group three questions:

1. What made you open the app this week?
2. What almost stopped you?
3. What would make you come back tomorrow?

With 15–50 users, these answers will beat a underpowered A/B test.

---

## What not to do

- Don't reveal the group standings early, even indirectly through "room split" before a pick.
- Don't make individual voter leaderboards public without opt-in; the private ranking is a core promise.
- Don't lean on loss aversion. Offer repair and recovery, not punishment.
- Don't add comments or chat without moderation. Reactions, superlatives, and share cards carry most of the social value with far less risk.
- Don't send daily notifications from multiple channels. One well-timed reminder plus the reveal is enough.
- Don't build a season archive until the data model is designed for it; but decide soon, because full reset currently destroys the club's history.

---

## Open questions for you

1. **Room split:** should the post-pick split be anonymous aggregate only, or show voter names? (Recommend aggregate only.)
2. **Daily goal:** keep the global goal for a shared ritual, or allow a personal goal per voter? (Recommend global first, personal override later.)
3. **Reminders:** is a Discord/WhatsApp webhook acceptable, or do you want to stay entirely in-app? A webhook is the fastest real retention win for a club.
4. **Archive:** will this club run more than one season? If yes, prioritize the archive data model before the next reset.
5. **Share cards:** is album art essential, or is the cassette/typography treatment enough? Text-only is safer for CORS and copyright.
6. **Admin curation:** how much do you want the admin to hand-pick (daily cut, theme nights, predictions), versus letting the engine run?

---

## Pick list

Reply with the IDs you want and I will turn them into a scoped implementation plan.

**Recommended first batch:** `B1`, `B2`, `A1`, `A2`, `C1`, `C2`, `C3`, `E1`, `F1`, `E4`

**Fun / social second batch:** `D1`, `D2`, `D4`, `D6`, `C5`, `C9`, `A8`, `F8`

**Big bets for later:** `E8`, `E9`, `C7`, `D7`, `F14`, `A6`
