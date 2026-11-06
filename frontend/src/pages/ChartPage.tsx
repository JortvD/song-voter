import { useCallback, useEffect, useRef, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'

import { MixtapeCase } from '../components/MixtapeCase'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { ChartStatus, ChartStories, PublicChart, User } from '../lib/types'

const PAGE_SIZE = 50

type Row = {
  id: string
  rank: number | null
  title: string
  artists: string
  album: string | null
  image: string | null
  score: number | null
  comparisons: number
  wins: number
  noData: boolean
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

function normalizeOfficial(entry: PublicChart['tracks'][number]): Row {
  return {
    id: entry.track.id,
    rank: entry.rank,
    title: entry.track.title,
    artists: entry.track.artists,
    album: entry.track.album,
    image: entry.track.album_image_url,
    score: entry.rating,
    comparisons: entry.comparisons,
    wins: entry.wins,
    noData: false,
  }
}

function StatPill({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border-2 border-line bg-paper px-3 py-2">
      <p className="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
        {label}
      </p>
      <p className="mt-1 font-serif text-xl leading-none">{value}</p>
    </div>
  )
}

function Cover({
  image,
  title,
  size = 'size-10',
}: {
  image: string | null
  title: string
  size?: string
}) {
  if (image) {
    return (
      <img
        src={image}
        alt={`${title} cover`}
        className={`${size} shrink-0 rounded-lg border-2 border-ink object-cover`}
        loading="lazy"
      />
    )
  }

  return (
    <div
      className={`${size} grid shrink-0 place-items-center rounded-lg border-2 border-ink bg-paper font-serif text-lg text-muted/60`}
    >
      ♪
    </div>
  )
}

function Podium({ rows }: { rows: Row[] }) {
  if (rows.length < 3) return null

  const [first, second, third] = rows
  const steps = [
    {
      row: second,
      rank: 2,
      order: 'order-2 sm:order-1 sm:mt-7',
      sticker: 'pv-sticker pv-sticker-paper',
    },
    {
      row: first,
      rank: 1,
      order: 'order-1 sm:order-2',
      sticker: 'pv-sticker',
    },
    {
      row: third,
      rank: 3,
      order: 'order-3 sm:mt-12',
      sticker: 'pv-sticker pv-sticker-butter',
    },
  ]

  return (
    <section className="mt-7">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
        <div>
          <p className="pv-label">Top 3</p>
          <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">The podium</h2>
        </div>
        <span className="pv-label">The first names on the J-card</span>
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-3 sm:items-start">
        {steps.map(({ row, rank, order, sticker }) => (
          <article
            key={row.id}
            className={`pv-card p-4 ${order} ${
              rank === 1
                ? 'border-[var(--playlist-accent)] shadow-[6px_6px_0_0_var(--playlist-accent)]'
                : ''
            }`}
          >
            <div className="flex items-start justify-between gap-2">
              <span className={sticker}>#{rank}</span>
              {rank === 1 && (
                <span className="pv-sticker pv-sticker-butter">Top of the pile</span>
              )}
            </div>

            <div className="mt-3 flex items-center gap-3 sm:flex-col sm:items-start">
              <Cover image={row.image} title={row.title} size="size-16" />
              <div className="min-w-0">
                <p className="line-clamp-2 font-serif text-lg leading-tight tracking-[-0.02em]">
                  {row.title}
                </p>
                <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                  {row.artists}
                </p>
              </div>
            </div>

            <div className="mt-3 flex items-baseline justify-between border-t-2 border-ink/10 pt-2">
              <span className="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
                Score
              </span>
              <span className="font-serif text-2xl leading-none">
                {row.score === null ? '—' : Math.round(row.score)}
              </span>
            </div>
          </article>
        ))}
      </div>
    </section>
  )
}

function StoryCards({ stories }: { stories: ChartStories | null }) {
  if (!stories) return null

  const cards = []

  if (stories.most_duelled) {
    const story = stories.most_duelled
    cards.push(
      <article key="duelled" className="pv-card p-4">
        <div className="flex items-start justify-between gap-2">
          <span className="pv-sticker pv-sticker-cobalt">Most duelled</span>
          <span className="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(story.comparisons)} duels
          </span>
        </div>
        <div className="mt-3 flex items-center gap-3">
          <Cover image={story.track.album_image_url} title={story.track.title} size="size-14" />
          <div className="min-w-0">
            <p className="truncate font-serif text-lg leading-tight">{story.track.title}</p>
            <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              {story.track.artists}
            </p>
          </div>
        </div>
        <p className="mt-3 text-sm leading-relaxed text-muted">
          This one has been in the most head-to-heads.
        </p>
      </article>,
    )
  }

  if (stories.top_voter) {
    const story = stories.top_voter
    cards.push(
      <article key="voter" className="pv-card p-4">
        <div className="flex items-start justify-between gap-2">
          <span className="pv-sticker pv-sticker-butter">Top voter</span>
          <span className="font-mono text-[8px] uppercase tracking-[0.14em] text-muted">
            {formatNumber(story.picks)} picks
          </span>
        </div>
        <div className="mt-3 flex items-center gap-3">
          <span className="grid size-14 shrink-0 place-items-center rounded-full border-2 border-ink bg-[var(--playlist-accent)] font-serif text-2xl text-[var(--playlist-accent-ink)]">
            {story.username.slice(0, 1).toUpperCase()}
          </span>
          <div className="min-w-0">
            <p className="truncate font-serif text-lg leading-tight">{story.username}</p>
            <p className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              Most active listener
            </p>
          </div>
        </div>
        <p className="mt-3 text-sm leading-relaxed text-muted">
          The most picks in the current season.
        </p>
      </article>,
    )
  }

  if (cards.length === 0) return null

  return (
    <section className="mt-8">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
        <div>
          <p className="pv-label">The stories</p>
          <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">Liner notes</h2>
        </div>
        <span className="pv-label">A few things the picks revealed</span>
      </div>
      <div className="mt-4 grid gap-4 md:grid-cols-2">{cards}</div>
    </section>
  )
}

function SealedState({
  status,
  user,
}: {
  status: ChartStatus
  user: User | null
}) {
  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-4">
        <div>
          <p className="pv-label">Public</p>
          <h1 className="mt-1 font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
            The Standings are sealed.
          </h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs · {formatNumber(status.voters)} voters
          </p>
        </div>
        {user && (
          <Link to="/duel" className="btn">
            Next duel
          </Link>
        )}
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-[1.05fr_0.95fr] lg:items-center">
        <section className="pv-card p-6 sm:p-8">
          <span className="pv-sticker">Sealed</span>
          <h2 className="mt-4 font-serif text-3xl leading-tight tracking-[-0.03em]">
            The Reveal hasn&apos;t happened yet.
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-muted">
            Picking stays open. The group&apos;s shared standings are being built
            in private, and the admin will open The Reveal when it&apos;s time.
          </p>

          <div className="mt-5 grid grid-cols-3 gap-2">
            <StatPill label="Picks" value={formatNumber(status.votes_cast)} />
            <StatPill label="Songs" value={formatNumber(status.total_tracks)} />
            <StatPill label="Voters" value={formatNumber(status.voters)} />
          </div>

          <div className="mt-6 flex flex-wrap gap-3">
            <Link to={user ? '/duel' : '/#auth'} className="btn">
              {user ? 'Next duel' : 'Sign in to pick'}
            </Link>
            {user && (
              <Link to="/your-ranking" className="btn-ghost">
                Your Ranking
              </Link>
            )}
          </div>
        </section>

        <MixtapeCase
          title={status.playlist_name ?? 'No tape yet'}
          coverUrl={status.playlist_image_url}
          trackCount={status.total_tracks}
          description={status.playlist_description}
          revealed={false}
        />
      </div>
    </>
  )
}

function NoPlaylistState({ user }: { user: User | null }) {
  return (
    <section className="pv-card p-6 text-center sm:p-10">
      <span className="pv-sticker pv-sticker-paper">No tape yet</span>
      <h1 className="mt-4 font-serif text-4xl leading-tight tracking-[-0.03em]">
        The playlist isn&apos;t here yet.
      </h1>
      <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
        The admin hasn&apos;t imported a playlist yet, so there is nothing to
        reveal.
      </p>
      <div className="mt-6 flex flex-wrap justify-center gap-3">
        {user?.is_admin && (
          <Link to="/admin" className="btn">
            Import a tape
          </Link>
        )}
        <Link to="/" className="btn-ghost">
          Back home
        </Link>
      </div>
    </section>
  )
}

export function ChartPage() {
  const { user } = useAuth()
  const { status, loading: playlistLoading, refresh: refreshPlaylist } = usePlaylist()
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [rows, setRows] = useState<Row[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [stories, setStories] = useState<ChartStories | null>(null)
  const [reloadKey, setReloadKey] = useState(0)
  const [showReveal, setShowReveal] = useState(false)
  const sentinelRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    if (!status?.revealed) {
      setRows([])
      setTotal(0)
      setLoading(false)
      return
    }

    let active = true
    setLoading(true)
    setError(null)
    setRows([])
    setTotal(0)

    const load = async () => {
      try {
        const data = await api.getPublicChart(0, PAGE_SIZE, query)
        if (!active) return
        setRows(data.tracks.map(normalizeOfficial))
        setTotal(data.total)
      } catch (caught) {
        if (!active) return
        if (caught instanceof ApiError && caught.status === 403) {
          setError('The standings were sealed again. Refreshing…')
          await refreshPlaylist()
        } else {
          setError(
            caught instanceof ApiError
              ? caught.message
              : 'Could not load the standings',
          )
        }
      } finally {
        if (active) setLoading(false)
      }
    }

    void load()

    return () => {
      active = false
    }
  }, [status?.revealed, query, reloadKey, refreshPlaylist])

  useEffect(() => {
    if (!status?.revealed) {
      setStories(null)
      return
    }

    let active = true
    api
      .getChartStories()
      .then((data) => {
        if (active) setStories(data)
      })
      .catch(() => {
        if (active) setStories(null)
      })

    return () => {
      active = false
    }
  }, [status?.revealed, reloadKey])

  useEffect(() => {
    if (!status?.revealed) {
      setShowReveal(false)
      return
    }

    if (showReveal) return

    const storageKey = `pv-reveal-seen:${status?.playlist_id ?? 'active'}`
    let seen = false
    try {
      seen = window.sessionStorage.getItem(storageKey) === '1'
    } catch {
      seen = false
    }

    if (seen) return

    setShowReveal(true)
    try {
      window.sessionStorage.setItem(storageKey, '1')
    } catch {
      // Storage can be disabled; the moment still plays for this load.
    }
  }, [status?.revealed, status?.playlist_id, showReveal])

  useEffect(() => {
    if (!showReveal) return

    const timer = window.setTimeout(() => setShowReveal(false), 2600)
    return () => window.clearTimeout(timer)
  }, [showReveal])

  const loadMore = useCallback(async () => {
    if (loading || loadingMore || rows.length >= total) return

    setLoadingMore(true)
    setError(null)

    try {
      const data = await api.getPublicChart(rows.length, PAGE_SIZE, query)
      setRows((current) => [...current, ...data.tracks.map(normalizeOfficial)])
      setTotal(data.total)
    } catch (caught) {
      if (caught instanceof ApiError && caught.status === 403) {
        setError('The standings were sealed again. Refreshing…')
        await refreshPlaylist()
      } else {
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load more songs',
        )
      }
    } finally {
      setLoadingMore(false)
    }
  }, [loading, loadingMore, rows.length, total, query, refreshPlaylist])

  useEffect(() => {
    const node = sentinelRef.current
    if (
      !node ||
      !status?.revealed ||
      loading ||
      loadingMore ||
      rows.length >= total
    ) {
      return
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          void loadMore()
        }
      },
      { rootMargin: '400px 0px' },
    )

    observer.observe(node)
    return () => observer.disconnect()
  }, [status?.revealed, loading, loadingMore, rows.length, total, loadMore])

  function refresh() {
    setReloadKey((value) => value + 1)
    void refreshPlaylist()
  }

  function applySearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setQuery(queryInput.trim())
  }

  function clearSearch() {
    setQueryInput('')
    setQuery('')
  }

  if (playlistLoading && !status) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
        <div className="h-28 animate-pulse rounded-2xl border-2 border-line bg-card" />
        <div className="mt-6 space-y-2">
          {[0, 1, 2, 3, 4, 5].map((index) => (
            <div
              key={index}
              className="h-16 animate-pulse rounded-xl border-2 border-line bg-card"
            />
          ))}
        </div>
      </main>
    )
  }

  if (!status?.playlist_name) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <NoPlaylistState user={user} />
      </main>
    )
  }

  if (!status.revealed) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
        <SealedState status={status} user={user} />
      </main>
    )
  }

  const ratedRows = rows.filter((row) => !row.noData)
  const podium = !query && ratedRows.length >= 3 ? ratedRows.slice(0, 3) : []
  const podiumIds = new Set(podium.map((row) => row.id))
  const listRows = podium.length > 0 ? rows.filter((row) => !podiumIds.has(row.id)) : rows

  return (
    <main className="pv-rise mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      {showReveal && (
        <div className="pv-seal pointer-events-none fixed inset-x-0 top-24 z-50 flex justify-center px-4">
          <div className="flex items-center gap-3 rounded-full border-2 border-ink bg-card px-4 py-2 shadow-[4px_4px_0_0_var(--playlist-accent)]">
            <span className="grid size-10 place-items-center rounded-full border-2 border-ink bg-[var(--playlist-accent)] font-mono text-[8px] font-semibold uppercase tracking-[0.08em] text-[var(--playlist-accent-ink)]">
              Seal
            </span>
            <span>
              <span className="block font-mono text-[9px] uppercase tracking-[0.16em] text-muted">
                The Reveal is open
              </span>
              <span className="block font-serif text-lg leading-tight">
                The standings are live.
              </span>
            </span>
          </div>
        </div>
      )}

      <header className="border-b-2 border-ink/15 pb-4">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <span className="pv-label">Public</span>
              <span className="pv-sticker">Now voting</span>
              <span className="pv-sticker pv-sticker-butter">Reveal open</span>
              <span className="pv-label">{status.playlist_name}</span>
            </div>
            <h1 className="mt-2 font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
              The Standings.
            </h1>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {formatNumber(status.votes_cast)} picks ·{' '}
              {formatNumber(status.total_tracks)} songs ·{' '}
              {formatNumber(status.voters)} voters · {formatNumber(status.duels)} duels
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <button type="button" onClick={refresh} className="btn-ghost">
              Refresh
            </button>
            {user && (
              <Link to="/duel" className="btn">
                Duel
              </Link>
            )}
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <span className="pv-sticker pv-sticker-paper">Official</span>
          <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            Every pick counts once
          </span>
        </div>

        <form onSubmit={applySearch} className="mt-4 flex flex-col gap-2 sm:flex-row">
          <input
            type="search"
            value={queryInput}
            onChange={(event) => setQueryInput(event.target.value)}
            placeholder="Search songs, artists or albums…"
            className="pv-input flex-1"
            aria-label="Search the standings"
          />
          <button type="submit" className="btn shrink-0">
            Search
          </button>
          {query && (
            <button type="button" onClick={clearSearch} className="btn-ghost shrink-0">
              Clear
            </button>
          )}
        </form>

        {error && (
          <p
            role="alert"
            className="mt-4 rounded-xl border-2 border-danger bg-danger/5 px-3 py-2 text-sm text-danger"
          >
            {error}
          </p>
        )}
      </header>

      {!query && <StoryCards stories={stories} />}

      {loading ? (
        <div className="mt-6 space-y-2">
          {[0, 1, 2, 3, 4, 5].map((index) => (
            <div
              key={index}
              className="h-16 animate-pulse rounded-xl border-2 border-line bg-card"
            />
          ))}
        </div>
      ) : rows.length > 0 ? (
        <>
          <Podium rows={podium} />

          <section className="mt-7" aria-busy={loadingMore}>
            <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
              <div>
                <p className="pv-label">Official pressing</p>
                <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
                  The tracklist
                </h2>
              </div>
              <span className="pv-label">
                {query
                  ? `${formatNumber(total)} matching songs`
                  : `${formatNumber(total)} songs`}
              </span>
            </div>

            <div className="mt-3 overflow-hidden rounded-2xl border-2 border-ink bg-card">
              <div className="hidden grid-cols-[2.5rem_2.5rem_1fr_auto] items-center gap-3 border-b-2 border-ink px-3 py-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted sm:grid">
                <span>#</span>
                <span />
                <span>Song</span>
                <span className="text-right">Wins · Duels · Score</span>
              </div>

              <ol>
                {listRows.map((row) => (
                  <li
                    key={row.id}
                    className="grid grid-cols-[2.5rem_2.5rem_1fr] items-center gap-3 border-b border-line px-3 py-3 last:border-b-0 sm:grid-cols-[2.5rem_2.5rem_1fr_auto]"
                  >
                    <span className="font-serif text-xl leading-none text-muted">
                      {row.rank ? String(row.rank).padStart(2, '0') : '—'}
                    </span>

                    <Cover image={row.image} title={row.title} />

                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium leading-tight">
                        {row.title}
                      </p>
                      <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                        {row.artists}
                      </p>
                    </div>

                    <div className="col-span-3 flex flex-wrap items-center justify-end gap-3 font-mono text-[9px] uppercase tracking-[0.1em] text-muted sm:col-span-1 sm:gap-4">
                      <span>
                        <span className="text-ink">{row.wins}</span> W
                      </span>
                      <span className="hidden sm:inline">
                        <span className="text-ink">{row.comparisons}</span> D
                      </span>
                      {row.noData || row.score === null ? (
                        <span className="rounded border border-line px-1.5 py-0.5 text-[8px] uppercase tracking-[0.12em] text-muted">
                          No data
                        </span>
                      ) : (
                        <span className="font-serif text-lg leading-none text-ink">
                          {Math.round(row.score)}
                        </span>
                      )}
                    </div>
                  </li>
                ))}
              </ol>
            </div>

            <div ref={sentinelRef} className="h-1" />

            {loadingMore && (
              <div className="mt-4 space-y-2">
                {[0, 1, 2].map((index) => (
                  <div
                    key={index}
                    className="h-14 animate-pulse rounded-xl border-2 border-line bg-card"
                  />
                ))}
              </div>
            )}

            {!loadingMore && rows.length < total && (
              <button
                type="button"
                onClick={() => void loadMore()}
                className="btn-ghost mt-4 w-full"
              >
                Load more
              </button>
            )}

            <p
              role="status"
              aria-live="polite"
              className="mt-4 text-center font-mono text-[9px] uppercase tracking-[0.14em] text-muted"
            >
              Showing {formatNumber(rows.length)} of {formatNumber(total)} songs
              {rows.length >= total ? ' · that’s the whole tape' : ''}
            </p>
          </section>
        </>
      ) : (
        <section className="pv-card mt-6 p-6 text-center">
          <p className="text-sm text-muted">
            {query ? `No songs match “${query}”.` : 'The standings are empty.'}
          </p>
        </section>
      )}
    </main>
  )
}
