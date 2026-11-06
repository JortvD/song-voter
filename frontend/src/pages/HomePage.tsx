import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { Link } from 'react-router-dom'

import { AuthCard } from '../components/AuthCard'
import { MixtapeCase } from '../components/MixtapeCase'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { api } from '../lib/api'
import type { PersonalRanking, User } from '../lib/types'

function formatNumber(value: number) {
  return value.toLocaleString()
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

function LinerNote({
  number,
  title,
  children,
}: {
  number: string
  title: string
  children: ReactNode
}) {
  return (
    <article className="pv-note">
      <span className="pv-label">{number}</span>
      <h3 className="mt-2 font-serif text-xl tracking-[-0.02em]">{title}</h3>
      <div className="mt-2 text-sm leading-relaxed text-muted">{children}</div>
    </article>
  )
}

function YourSide({
  user,
  personal,
  loading,
}: {
  user: User
  personal: PersonalRanking | null
  loading: boolean
}) {
  const top = personal?.tracks[0]

  return (
    <section className="pv-card overflow-hidden">
      <div className="flex items-center justify-between gap-3 border-b-2 border-ink bg-ink px-5 py-3 text-paper">
        <div className="flex min-w-0 items-center gap-2">
          <span className="pv-sticker pv-sticker-butter px-1.5 py-0.5 text-[9px]">
            Side A
          </span>
          <span className="truncate font-serif text-lg leading-none tracking-[-0.02em]">
            Your side of the story
          </span>
        </div>
        <span className="shrink-0 font-mono text-[9px] uppercase tracking-[0.16em] text-paper/85">
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
              {top.track.title}
            </p>
            <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              {top.track.artists}
            </p>
          </div>
        ) : (
          <p className="mt-4 rounded-xl border-2 border-dashed border-line px-3 py-3 text-sm leading-relaxed text-muted">
            Your tape is blank. A few duels will fill Side A.
          </p>
        )}

        <div className="mt-5 flex flex-wrap gap-2">
          <Link to="/duel" className="btn">
            Start a duel
          </Link>
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
      title: 'Listen to two songs',
      body: 'Press Space to play and switch sides. Use 1 to pick Side A, or 2 to pick Side B.',
    },
    {
      title: 'Build Your Ranking',
      body: 'Every pick updates your private list right away. No one else can see it.',
    },
    {
      title: 'Open The Reveal',
      body: "The group's shared list stays sealed until the admin opens it. Then you can see the standings.",
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
    <section className="pv-card p-5">
      <div className="flex items-center justify-between gap-3 border-b-2 border-ink/15 pb-3">
        <h2 className="font-serif text-2xl tracking-[-0.02em]">How it works</h2>
        <span className="pv-sticker pv-sticker-paper">The short version</span>
      </div>

      <p className="mt-3 text-sm leading-relaxed text-muted">
        Every duel is a small vote. Pick the song you prefer, and the group&apos;s
        picks become the standings.
      </p>

      <ol className="mt-4 space-y-4">
        {steps.map((step, index) => (
          <li key={step.title} className="flex gap-3">
            <span className="grid size-8 shrink-0 place-items-center rounded-full border-2 border-ink bg-[var(--playlist-accent)] font-mono text-[10px] font-semibold text-[var(--playlist-accent-ink)] shadow-[2px_2px_0_0_#141a2a]">
              {index + 1}
            </span>
            <div>
              <h3 className="font-serif text-lg leading-tight tracking-[-0.02em]">
                {step.title}
              </h3>
              <p className="mt-1 text-sm leading-relaxed text-muted">{step.body}</p>
            </div>
          </li>
        ))}
      </ol>

      <dl className="mt-5 space-y-2 border-t-2 border-ink/15 pt-4">
        {glossary.map(({ term, definition }) => (
          <div key={term} className="flex flex-col gap-0.5 sm:flex-row sm:gap-2">
            <dt className="shrink-0 font-mono text-[9px] uppercase tracking-[0.12em] text-ink">
              {term}
            </dt>
            <dd className="text-sm leading-relaxed text-muted">{definition}</dd>
          </div>
        ))}
      </dl>
    </section>
  )
}

export function HomePage() {
  const { user } = useAuth()
  const { status, loading: playlistLoading } = usePlaylist()
  const [personal, setPersonal] = useState<PersonalRanking | null>(null)
  const [personalLoading, setPersonalLoading] = useState(false)

  const hasPlaylist = Boolean(status?.playlist_name)
  const playlistName = status?.playlist_name ?? 'No tape yet'
  const revealed = Boolean(status?.revealed)

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
    setPersonalLoading(true)

    api
      .getPersonalRanking(0, 1)
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
    }
  }, [user])

  return (
    <main className="flex-1">
      <section className="relative overflow-hidden border-b-2 border-ink/15">
        <div className="pointer-events-none absolute -right-32 top-10 size-96 rounded-full border-2 border-ink/10 pv-halftone opacity-[0.07]" />
        <div className="pointer-events-none absolute -left-20 bottom-5 hidden size-56 rotate-12 rounded-xl border-2 border-ink/10 pv-tape-strip opacity-40 lg:block" />

        <div className="relative mx-auto grid w-full max-w-6xl gap-10 px-4 py-12 sm:px-6 lg:grid-cols-[1.05fr_0.95fr] lg:items-center lg:px-8 lg:py-16">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="pv-sticker">
                {playlistLoading ? 'Tuning the deck' : hasPlaylist ? 'Now voting' : 'No tape yet'}
              </span>
              <span className="pv-label">
                {hasPlaylist ? playlistName : 'The current season'}
              </span>
            </div>

            <h1 className="mt-5 font-serif text-5xl leading-[0.88] tracking-[-0.055em] sm:text-7xl lg:text-8xl">
              Two songs.
              <br />
              One pick.
            </h1>

            <p className="mt-5 max-w-md text-base leading-relaxed text-muted">
              A playlist becomes a contest. Hear two songs, pick the one you
              prefer, and Your Ranking updates right away. The group&apos;s shared
              standings stay sealed until The Reveal.
            </p>

            <div className="mt-7 flex flex-wrap items-center gap-3">
              {user ? (
                <>
                  <Link to={hasPlaylist ? '/duel' : user.is_admin ? '/admin' : '/standings'} className="btn">
                    {hasPlaylist ? 'Start a duel' : user.is_admin ? 'Import a tape' : 'See the standings'}
                  </Link>
                  {hasPlaylist && (
                    <Link to="/your-ranking" className="btn-ghost">
                      Your Ranking
                    </Link>
                  )}
                  <Link to="/standings" className="btn-ghost">
                    The Standings
                  </Link>
                </>
              ) : (
                <>
                  <a href="#auth" className="btn">
                    Sign in to pick
                  </a>
                  <Link to="/standings" className="btn-ghost">
                    See the standings
                  </Link>
                </>
              )}
            </div>

            <div className="mt-8 grid grid-cols-2 gap-2 sm:grid-cols-4">
              <Stat
                label="Picks"
                value={playlistLoading ? '—' : formatNumber(status?.votes_cast ?? 0)}
                hint="and counting"
              />
              <Stat
                label="Duels"
                value={playlistLoading ? '—' : formatNumber(status?.duels ?? 0)}
                hint="head to head"
              />
              <Stat
                label="Voters"
                value={playlistLoading ? '—' : formatNumber(status?.voters ?? 0)}
                hint="in the room"
              />
              <Stat
                label="Songs covered"
                value={playlistLoading ? '—' : formatNumber(status?.tracks_compared ?? 0)}
                hint={`of ${formatNumber(status?.total_tracks ?? 0)}`}
              />
            </div>
          </div>

          <div className="relative mx-auto w-full max-w-md lg:ml-auto">
            <MixtapeCase
              title={playlistLoading ? 'Loading the tape…' : playlistName}
              coverUrl={status?.playlist_image_url ?? null}
              trackCount={status?.total_tracks ?? 0}
              description={status?.playlist_description}
              revealed={revealed}
            />

            <div className="mt-5 flex flex-wrap items-center justify-between gap-3 px-1">
              <span className="pv-label">
                {revealed ? 'The Reveal is open' : 'Sealed until The Reveal'}
              </span>
              <Link
                to="/standings"
                className="pv-link font-mono text-[9px] font-semibold uppercase tracking-[0.14em]"
              >
                {revealed ? 'Open the standings' : 'Peek at the seal'}
              </Link>
            </div>
          </div>
        </div>
      </section>

      <section className="mx-auto w-full max-w-6xl px-4 py-12 sm:px-6 lg:px-8">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-3">
          <div>
            <p className="pv-label">The current season</p>
            <h2 className="mt-1 font-serif text-3xl tracking-[-0.03em]">Liner notes</h2>
          </div>
          <span className="pv-sticker pv-sticker-paper">
            {hasPlaylist ? playlistName : 'No tape yet'}
          </span>
        </div>

        <div className="mt-6 grid gap-5 md:grid-cols-3">
          <LinerNote number="01" title="The tape">
            <p className="line-clamp-4">
              {status?.playlist_description?.trim() ||
                'A shared playlist, a stack of duels, and one group’s taste on the line.'}
            </p>
            <p className="mt-3 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {formatNumber(status?.total_tracks ?? 0)} songs on Side A
            </p>
          </LinerNote>

          <LinerNote number="02" title="The tally">
            <dl className="space-y-1.5 font-mono text-[9px] uppercase tracking-[0.12em]">
              <div className="flex items-center justify-between gap-3 border-b border-dashed border-line pb-1">
                <dt>Picks</dt>
                <dd className="text-ink">{formatNumber(status?.votes_cast ?? 0)}</dd>
              </div>
              <div className="flex items-center justify-between gap-3 border-b border-dashed border-line pb-1">
                <dt>Duels</dt>
                <dd className="text-ink">{formatNumber(status?.duels ?? 0)}</dd>
              </div>
              <div className="flex items-center justify-between gap-3">
                <dt>Voters</dt>
                <dd className="text-ink">{formatNumber(status?.voters ?? 0)}</dd>
              </div>
            </dl>
          </LinerNote>

          <LinerNote number="03" title="The reveal">
            {revealed ? (
              <>
                The Reveal is open. You can now see the group&apos;s standings.
                <div className="mt-3">
                  <Link to="/standings" className="pv-link font-mono text-[9px] font-semibold uppercase tracking-[0.14em]">
                    Open the standings
                  </Link>
                </div>
              </>
            ) : (
              <>
                The standings stay sealed until the admin opens The Reveal.
                {user
                  ? ' Your own ranking is always yours to read.'
                  : ' Sign in to start your own tape.'}
                <div className="mt-3">
                  {user ? (
                    <Link
                      to="/your-ranking"
                      className="pv-link font-mono text-[9px] font-semibold uppercase tracking-[0.14em]"
                    >
                      Open Your Ranking
                    </Link>
                  ) : (
                    <a
                      href="#auth"
                      className="pv-link font-mono text-[9px] font-semibold uppercase tracking-[0.14em]"
                    >
                      Sign in
                    </a>
                  )}
                </div>
              </>
            )}
          </LinerNote>
        </div>
      </section>

      <section className="mx-auto w-full max-w-6xl px-4 pb-16 sm:px-6 lg:px-8">
        <div className="grid items-start gap-6 lg:grid-cols-[1.05fr_0.95fr]">
          <div className="order-2 lg:order-1">
            {user ? (
              <YourSide user={user} personal={personal} loading={personalLoading} />
            ) : (
              <AuthCard />
            )}
          </div>
          <div className="order-1 lg:order-2">
            <HowItWorks />
          </div>
        </div>
      </section>
    </main>
  )
}
