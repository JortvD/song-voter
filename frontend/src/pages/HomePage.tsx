import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'

import { AuthCard } from '../components/AuthCard'
import { Countdown } from '../components/Countdown'
import { MixtapeCase } from '../components/MixtapeCase'
import { StreakCard } from '../components/streak/StreakCard'
import { StreakFlame } from '../components/streak/StreakFlame'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { api } from '../lib/api'
import { localTimezone } from '../lib/time'
import type { ChartStatus, PersonalSummary, User } from '../lib/types'

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatWhen(value: string | null) {
  if (!value) return 'soon'
  return new Date(value).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

function formatDate(value: string): string {
  return new Date(value).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function Stat({
  label,
  value,
  hint,
}: {
  label: string
  value: string
  hint?: string
}) {
  return (
    <div className="pv-stat">
      <span className="font-mono text-[9px] font-medium uppercase tracking-[0.16em] text-muted">
        {label}
      </span>
      <span className="mt-1 font-serif text-2xl leading-none tracking-[-0.02em]">
        {value}
      </span>
      {hint && (
        <span className="mt-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {hint}
        </span>
      )}
    </div>
  )
}

function TapeCounter({
  status,
  loading,
}: {
  status: ChartStatus | null
  loading: boolean
}) {
  const cells = [
    {
      label: 'Picks',
      value: loading ? '—' : formatNumber(status?.votes_cast ?? 0),
      hint: 'and counting',
    },
    {
      label: 'Duels',
      value: loading ? '—' : formatNumber(status?.duels ?? 0),
      hint: 'head to head',
    },
    {
      label: 'Voters',
      value: loading ? '—' : formatNumber(status?.voters ?? 0),
      hint: 'in the room',
    },
    {
      label: 'Coverage',
      value: loading ? '—' : formatNumber(status?.tracks_compared ?? 0),
      hint: `of ${formatNumber(status?.total_tracks ?? 0)} songs`,
    },
  ]

  return (
    <div className="pv-counter-strip">
      <div className="pv-counter-head">
        <span className="pv-counter-label">Tape counter</span>
        <span className="pv-counter-light" aria-hidden="true" />
      </div>
      <div className="pv-counter-cells">
        {cells.map((cell) => (
          <div key={cell.label} className="pv-counter-cell">
            <span className="pv-counter-cell-label">{cell.label}</span>
            <span className="pv-counter-cell-value">{cell.value}</span>
            <span className="pv-counter-cell-hint">{cell.hint}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

function Action({
  href,
  className,
  children,
}: {
  href: string
  className: string
  children: ReactNode
}) {
  if (href.startsWith('#')) {
    return (
      <a href={href} className={className}>
        {children}
      </a>
    )
  }

  return (
    <Link to={href} className={className}>
      {children}
    </Link>
  )
}

function YourSide({
  user,
  personal,
  loading,
}: {
  user: User
  personal: PersonalSummary | null
  loading: boolean
}) {
  const top = personal?.top_track

  return (
    <section className="pv-card min-w-0 overflow-hidden">
      <div className="flex min-w-0 items-center justify-between gap-3 border-b-2 border-ink bg-ink px-5 py-3 text-paper">
        <div className="flex min-w-0 items-center gap-2">
          <span className="pv-sticker pv-sticker-butter px-1.5 py-0.5 text-[9px]">
            Side A
          </span>
          <span className="min-w-0 truncate font-serif text-lg leading-none tracking-[-0.02em]">
            Your side of the story
          </span>
        </div>
        <span className="hidden shrink-0 font-mono text-[9px] uppercase tracking-[0.16em] text-paper/85 sm:inline">
          Member
        </span>
      </div>

      <div className="p-5">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <p className="pv-label">Signed in as</p>
            <h2 className="mt-1 truncate font-serif text-3xl leading-none tracking-[-0.03em]">
              {user.username}
            </h2>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              Member since {formatDate(user.created_at)}
            </p>
          </div>
          <span className="pv-sticker pv-sticker-paper shrink-0">
            {user.is_admin ? 'Admin' : 'Voter'}
          </span>
        </div>

        <div className="mt-5 grid grid-cols-2 gap-2">
          <Stat
            label="Your picks"
            value={loading ? '—' : formatNumber(personal?.votes ?? 0)}
            hint="and counting"
          />
          <Stat
            label="Songs ranked"
            value={loading ? '—' : formatNumber(personal?.total ?? 0)}
            hint="on your tape"
          />
        </div>

        {loading ? (
          <div className="mt-4 h-16 animate-pulse rounded-xl border-2 border-line bg-paper" />
        ) : top ? (
          <div className="mt-4 rounded-xl border-2 border-line bg-paper p-3">
            <p className="pv-label">Your current #1</p>
            <p className="mt-1 truncate font-serif text-lg leading-tight">
              {top.title}
            </p>
            <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              {top.artists}
            </p>
          </div>
        ) : (
          <p className="mt-4 rounded-xl border-2 border-dashed border-line px-3 py-3 text-sm leading-relaxed text-muted">
            Your tape is blank. A few duels will fill Side A.
          </p>
        )}

        <div className="mt-5 flex flex-wrap gap-2">
          <Link to="/your-ranking" className="btn-ghost">
            Your Ranking
          </Link>
          {user.is_admin && (
            <Link to="/admin" className="btn-ghost">
              Mission control
            </Link>
          )}
        </div>
      </div>
    </section>
  )
}

function HowItWorks() {
  const steps = [
    {
      title: 'Hear two songs',
      body: 'Play both sides, then pick the one you prefer.',
    },
    {
      title: 'Build Your Ranking',
      body: 'Every pick updates your private list right away.',
    },
    {
      title: 'Open The Reveal',
      body: "The group's shared list stays sealed until the admin opens it.",
    },
  ]

  const glossary = [
    {
      term: 'Your Ranking',
      definition: 'Your private list. Only you can see it.',
    },
    {
      term: 'The Standings',
      definition: "The group's shared list.",
    },
    {
      term: 'The Reveal',
      definition: 'The moment the group list goes public.',
    },
  ]

  return (
    <section className="pv-card min-w-0 p-5">
      <div className="flex min-w-0 flex-wrap items-center justify-between gap-3 border-b-2 border-ink/15 pb-3">
        <div>
          <p className="pv-label">The short version</p>
          <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
            How it works
          </h2>
        </div>
        <span className="pv-sticker pv-sticker-paper">Three steps</span>
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        {steps.map((step, index) => (
          <div key={step.title} className="flex gap-2.5">
            <span className="grid size-7 shrink-0 place-items-center rounded-full border-2 border-ink bg-[var(--playlist-accent)] font-mono text-[9px] font-semibold text-[var(--playlist-accent-ink)] shadow-[2px_2px_0_0_#141a2a]">
              {index + 1}
            </span>
            <div className="min-w-0">
              <h3 className="font-serif text-base leading-tight tracking-[-0.02em]">
                {step.title}
              </h3>
              <p className="mt-1 text-xs leading-relaxed text-muted">
                {step.body}
              </p>
            </div>
          </div>
        ))}
      </div>

      <details className="group mt-4 border-t-2 border-ink/15 pt-3">
        <summary className="flex cursor-pointer list-none items-center justify-between gap-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted [&::-webkit-details-marker]:hidden">
          <span>What do the words mean?</span>
          <span className="text-base leading-none transition group-open:rotate-45">
            +
          </span>
        </summary>
        <dl className="mt-3 space-y-2">
          {glossary.map(({ term, definition }) => (
            <div key={term} className="flex flex-col gap-0.5 sm:flex-row sm:gap-2">
              <dt className="shrink-0 font-mono text-[9px] uppercase tracking-[0.12em] text-ink">
                {term}
              </dt>
              <dd className="text-sm leading-relaxed text-muted">{definition}</dd>
            </div>
          ))}
        </dl>
      </details>
    </section>
  )
}

export function HomePage() {
  const { user } = useAuth()
  const { status, loading: playlistLoading } = usePlaylist()
  const [personal, setPersonal] = useState<PersonalSummary | null>(null)
  const [personalLoading, setPersonalLoading] = useState(false)

  const hasPlaylist = Boolean(status?.playlist_name)
  const playlistName = status?.playlist_name ?? 'No tape yet'
  const revealed = Boolean(status?.revealed)
  const scheduled = Boolean(hasPlaylist && status && status.started_at === null)
  const votingPaused = Boolean(status?.voting_paused)
  const streak = personal?.streak ?? null
  const canVote = hasPlaylist && !scheduled && !votingPaused
  const streakRemaining = streak ? Math.max(0, streak.goal - streak.today_picks) : 0
  const countdownTarget = scheduled
    ? (status?.scheduled_start_at ?? null)
    : !revealed
      ? (status?.scheduled_reveal_at ?? null)
      : null
  const countdownLabel = scheduled ? 'Doors open in' : 'The seal opens in'

  const description =
    status?.playlist_description?.trim() ||
    'A shared playlist, a stack of duels, and one group’s taste on the line.'

  const roomCopy = !hasPlaylist
    ? 'The room is waiting for its first tape. The admin sets the playlist and the start time.'
    : scheduled
      ? 'The doors are still closed. The song list is fetched when the tape opens, so late additions make the cut.'
      : votingPaused
        ? 'The room is paused. Existing picks and rankings are safe.'
        : revealed
          ? 'The room has heard the tape. The group’s shared list is open.'
          : 'The room is open. Every pick is a small vote, and every vote counts once.'

  const roomMeta = !hasPlaylist
    ? 'No artists or albums yet'
    : scheduled
      ? 'Artists and albums appear when the tape opens'
      : `${formatNumber(status?.total_artists ?? 0)} artists · ${formatNumber(
          status?.total_albums ?? 0,
        )} albums`

  const sealCopy = !hasPlaylist
    ? 'There is nothing to seal yet. Once a tape is set and the votes come in, the admin can open The Reveal.'
    : revealed
      ? 'The seal is broken. The group’s shared list is public.'
      : scheduled
        ? 'The group list stays sealed. The countdown is for the doors, not the reveal.'
        : votingPaused
          ? 'The group list is sealed. Voting is paused until the admin resumes it.'
          : 'The group list is sealed until the admin opens The Reveal.'

  const primary = !hasPlaylist
    ? user?.is_admin
      ? { href: '/admin', label: 'Import a tape' }
      : user
        ? { href: '/standings', label: 'See the standings' }
        : { href: '#auth', label: 'Sign in to pick' }
    : scheduled
      ? { href: '/stats', label: 'See what’s coming' }
      : votingPaused
        ? { href: '/stats', label: 'See the stats' }
        : user
          ? { href: '/duel', label: 'Start a duel' }
          : { href: '#auth', label: 'Sign in to pick' }

  const secondary =
    user && hasPlaylist && !scheduled && !votingPaused
      ? { href: '/your-ranking', label: 'Your Ranking' }
      : hasPlaylist
        ? { href: '/standings', label: 'The Standings' }
        : { href: '#how-it-works', label: 'How it works' }

  useEffect(() => {
    if (window.location.hash !== '#auth' || user) return

    const timer = window.setTimeout(() => {
      document.getElementById('auth')?.scrollIntoView({ behavior: 'smooth', block: 'start' })
    }, 80)

    return () => window.clearTimeout(timer)
  }, [user])

  useEffect(() => {
    if (!user) {
      setPersonal(null)
      return
    }

    let active = true
    const controller = new AbortController()
    setPersonalLoading(true)

    api
      .getPersonalSummary(controller.signal, localTimezone())
      .then((data) => {
        if (active) setPersonal(data)
      })
      .catch(() => {
        if (active) setPersonal(null)
      })
      .finally(() => {
        if (active) setPersonalLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [user])

  return (
    <main className="flex-1">
      {user && canVote && streak && streak.at_risk && !streak.today_met && (
        <div className="border-b-2 border-ink/15 bg-[var(--playlist-accent-tint)]">
          <div className="mx-auto flex w-full max-w-6xl items-center gap-3 px-4 py-2 sm:px-6 lg:px-8">
            <span className="pv-streak-flame">
              <StreakFlame className="size-4" />
            </span>
            <p className="min-w-0 flex-1 text-xs leading-snug text-ink">
              <strong className="font-semibold">
                {streak.current}-day streak
              </strong>
              {' · '}
              {streakRemaining} more{' '}
              {streakRemaining === 1 ? 'pick' : 'picks'} today to keep it
              alive.
            </p>
            <Link
              to="/duel"
              className="shrink-0 font-mono text-[9px] font-semibold uppercase tracking-[0.12em] text-[var(--playlist-accent-strong)] underline decoration-2 underline-offset-4"
            >
              Keep it alive →
            </Link>
          </div>
        </div>
      )}

      <section className="relative overflow-hidden border-b-2 border-ink/15">
        <div className="pointer-events-none absolute -right-32 top-10 size-96 rounded-full border-2 border-ink/10 pv-halftone opacity-[0.07]" />
        <div className="pointer-events-none absolute -left-20 bottom-5 hidden size-56 rotate-12 rounded-xl border-2 border-ink/10 pv-tape-strip opacity-40 lg:block" />

        <div className="relative mx-auto grid w-full max-w-6xl gap-10 px-4 py-12 sm:px-6 lg:grid-cols-[1.05fr_0.95fr] lg:items-center lg:px-8 lg:py-16">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <span
                className={`pv-sticker ${
                  scheduled
                    ? 'pv-sticker-butter'
                    : votingPaused
                      ? 'pv-sticker-paper'
                      : ''
                }`}
              >
                {playlistLoading
                  ? 'Tuning the deck'
                  : !hasPlaylist
                    ? 'No tape yet'
                    : scheduled
                      ? 'Scheduled'
                      : votingPaused
                        ? 'Voting paused'
                        : 'Now voting'}
              </span>
              <span className="max-w-full break-words pv-label">
                {hasPlaylist ? playlistName : 'The current season'}
              </span>
            </div>

            <h1 className="mt-5 font-serif text-5xl leading-[0.88] tracking-[-0.055em] sm:text-7xl lg:text-8xl">
              Two songs.
              <br />
              One pick.
            </h1>

            <p className="mt-5 max-w-md text-base leading-relaxed text-muted">
              A shared playlist becomes a listening room. Hear two songs, pick
              one, and the group&apos;s taste slowly takes shape.
            </p>

            <div className="mt-7 flex flex-wrap items-center gap-3">
              <Action href={primary.href} className="btn">
                {primary.label}
              </Action>
              <Action href={secondary.href} className="btn-ghost">
                {secondary.label}
              </Action>
            </div>

            <TapeCounter status={status} loading={playlistLoading} />
          </div>

          <div className="relative mx-auto w-full max-w-md lg:ml-auto">
            <MixtapeCase
              title={playlistLoading ? 'Loading the tape…' : playlistName}
              coverUrl={status?.playlist_image_url ?? null}
              trackCount={status?.total_tracks ?? 0}
              description={status?.playlist_description}
              revealed={revealed}
              songsLabel={scheduled ? 'Scheduled' : undefined}
              statusLabel={scheduled ? 'Scheduled' : undefined}
              showDescription={false}
            />

            <div className="mt-5 flex flex-wrap items-center justify-between gap-3 px-1">
              <span className="pv-label">
                {!hasPlaylist
                  ? 'Waiting for a tape'
                  : scheduled
                    ? `Voting opens ${formatWhen(status?.scheduled_start_at ?? null)}`
                    : votingPaused
                      ? 'Voting is paused'
                      : revealed
                        ? 'The Reveal is open'
                        : 'Sealed until The Reveal'}
              </span>
              <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                Side A · Playlist Voter
              </span>
            </div>
          </div>
        </div>
      </section>

      {user && canVote && (personalLoading || streak) && (
        <section className="mx-auto w-full max-w-6xl px-4 pt-6 sm:px-6 lg:px-8">
          <StreakCard streak={streak} loading={personalLoading} />
        </section>
      )}

      <section className="mx-auto w-full max-w-6xl px-4 py-10 sm:px-6 lg:px-8">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-3">
          <div>
            <p className="pv-label">The current season</p>
            <h2 className="mt-1 font-serif text-3xl tracking-[-0.03em]">
              The tape notes
            </h2>
          </div>
          <span className="max-w-full truncate pv-sticker pv-sticker-paper">
            {hasPlaylist ? playlistName : 'No tape yet'}
          </span>
        </div>

        <article className="pv-tape-notes mt-7">
          <span className="pv-tape-notes-tab">Notes</span>

          <div className="pv-tape-notes-fold">
            <p className="pv-label">01 · The tape</p>
            <h3 className="mt-1 font-serif text-xl tracking-[-0.02em]">
              The tape
            </h3>
            <p className="mt-2 line-clamp-4 break-words text-sm leading-relaxed text-muted">
              {description}
            </p>
            <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {!hasPlaylist
                ? 'No tape set yet'
                : scheduled
                  ? `Scheduled · opens ${formatWhen(status?.scheduled_start_at ?? null)}`
                  : `${formatNumber(status?.total_tracks ?? 0)} songs on Side A`}
            </p>
          </div>

          <div className="pv-tape-notes-fold">
            <p className="pv-label">02 · The room</p>
            <h3 className="mt-1 font-serif text-xl tracking-[-0.02em]">
              The room
            </h3>
            <p className="mt-2 break-words text-sm leading-relaxed text-muted">
              {roomCopy}
            </p>
            <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {roomMeta}
            </p>
          </div>

          <div className="pv-tape-notes-fold">
            <p className="pv-label">03 · The seal</p>
            <h3 className="mt-1 font-serif text-xl tracking-[-0.02em]">
              The seal
            </h3>
            <p className="mt-2 break-words text-sm leading-relaxed text-muted">
              {sealCopy}
            </p>
            {countdownTarget && (
              <Countdown target={countdownTarget} label={countdownLabel} />
            )}
            <div className="mt-3">
              <Link
                to={revealed ? '/standings' : '/stats'}
                className="pv-link font-mono text-[9px] font-semibold uppercase tracking-[0.14em]"
              >
                {revealed ? 'Open the standings' : 'The Stats'}
              </Link>
            </div>
          </div>
        </article>
      </section>

      <section
        id="how-it-works"
        className="mx-auto w-full max-w-6xl px-4 pb-16 sm:px-6 lg:px-8"
      >
        <div className="grid items-start gap-6 lg:grid-cols-[1.05fr_0.95fr]">
          <div className="order-2 min-w-0 lg:order-1">
            {user ? (
              <YourSide user={user} personal={personal} loading={personalLoading} />
            ) : (
              <AuthCard />
            )}
          </div>
          <div className="order-1 min-w-0 lg:order-2">
            <HowItWorks />
          </div>
        </div>
      </section>
    </main>
  )
}
