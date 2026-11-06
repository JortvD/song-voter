import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'

import { Countdown } from '../components/Countdown'
import { MixtapeCase } from '../components/MixtapeCase'
import { TrackFilters } from '../components/TrackFilters'
import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { ChartStatus, PublicChart, User } from '../lib/types'

const PAGE_SIZE = 50

type Row = {
  id: string
  rank: number
  title: string
  artists: string
  image: string | null
  score: number
  comparisons: number
  wins: number
}

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

function normalizeOfficial(entry: PublicChart['tracks'][number]): Row {
  return {
    id: entry.track.id,
    rank: entry.rank,
    title: entry.track.title,
    artists: entry.track.artists,
    image: entry.track.album_image_url,
    score: entry.rating,
    comparisons: entry.comparisons,
    wins: entry.wins,
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
        <span className="pv-label">The first names on the sleeve</span>
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-3 sm:items-start">
        {steps.map(({ row, rank, order, sticker }) => (
          <article
            key={row.id}
            className={`pv-card min-w-0 p-4 ${order} ${
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

            <div className="mt-3 flex min-w-0 items-center gap-3 sm:flex-col sm:items-start">
              <Cover image={row.image} title={row.title} size="size-16" />
              <div className="min-w-0 max-w-full sm:w-full">
                <p className="line-clamp-2 break-words font-serif text-lg leading-tight tracking-[-0.02em] [overflow-wrap:anywhere]">
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
                {Math.round(row.score)}
              </span>
            </div>
          </article>
        ))}
      </div>
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
  const scheduled = status.started_at === null

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-4">
        <div className="min-w-0">
          <p className="pv-label">Public</p>
          <h1 className="mt-1 break-words font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
            {scheduled ? 'The tape is set.' : 'The Standings are sealed.'}
          </h1>
          <p className="mt-2 break-words font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs · {formatNumber(status.voters)} voters
          </p>
        </div>
        {scheduled ? (
          <Link to="/stats" className="btn">
            The Stats
          </Link>
        ) : (
          user && (
            <Link to="/duel" className="btn">
              Next duel
            </Link>
          )
        )}
      </div>

      <div className="mt-6 grid gap-6 lg:grid-cols-[1.05fr_0.95fr] lg:items-center">
        <section className="pv-card p-6 sm:p-8">
          <div className="flex flex-wrap gap-2">
            <span className={`pv-sticker ${scheduled ? 'pv-sticker-butter' : ''}`}>
              {scheduled ? 'Scheduled' : 'Sealed'}
            </span>
            {status.voting_paused && (
              <span className="pv-sticker pv-sticker-paper">Voting paused</span>
            )}
          </div>
          <h2 className="mt-4 font-serif text-3xl leading-tight tracking-[-0.03em]">
            {scheduled
              ? `Voting opens ${formatWhen(status.scheduled_start_at)}.`
              : 'The Reveal hasn\u2019t happened yet.'}
          </h2>
          <p className="mt-3 text-sm leading-relaxed text-muted">
            {scheduled
              ? 'You can log in now. The song list is fetched when the tape opens, so late additions to the Spotify playlist are included.'
              : 'Picking stays open. The group\u2019s shared standings are being built in private, and the admin will open The Reveal when it\u2019s time.'}
          </p>
          {status.scheduled_reveal_at && (
            <Countdown
              target={status.scheduled_reveal_at}
              label="The seal opens in"
            />
          )}
          {status.voting_paused && !scheduled && (
            <p className="mt-2 text-sm leading-relaxed text-muted">
              The admin has paused new picks. Existing picks and rankings are safe.
            </p>
          )}

          <div className="mt-5 grid grid-cols-3 gap-2">
            <StatPill label="Picks" value={formatNumber(status.votes_cast)} />
            <StatPill label="Songs" value={formatNumber(status.total_tracks)} />
            <StatPill label="Voters" value={formatNumber(status.voters)} />
          </div>

          <div className="mt-6 flex flex-wrap gap-3">
            {scheduled ? (
              <Link to="/" className="btn-ghost">
                Back home
              </Link>
            ) : (
              <>
                <Link to={user ? '/duel' : '/#auth'} className="btn">
                  {user ? 'Next duel' : 'Sign in to pick'}
                </Link>
                {user && (
                  <Link to="/your-ranking" className="btn-ghost">
                    Your Ranking
                  </Link>
                )}
              </>
            )}
            <Link to="/stats" className="btn-ghost">
              The Stats
            </Link>
          </div>
        </section>

        <MixtapeCase
          title={status.playlist_name ?? 'No tape yet'}
          coverUrl={status.playlist_image_url}
          trackCount={status.total_tracks}
          description={status.playlist_description}
          revealed={false}
          songsLabel={scheduled ? 'Scheduled' : undefined}
          statusLabel={scheduled ? 'Scheduled' : undefined}
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
  const [rows, setRows] = useState<Row[]>([])
  const [total, setTotal] = useState(0)
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [autoLoadPaused, setAutoLoadPaused] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [reloadKey, setReloadKey] = useState(0)
  const [showReveal, setShowReveal] = useState(false)
  const [searchInput, setSearchInput] = useState('')
  const [query, setQuery] = useState('')
  const [artistId, setArtistId] = useState<string | null>(null)
  const [decade, setDecade] = useState<number | null>(null)
  const [genre, setGenre] = useState<string | null>(null)
  const sentinelRef = useRef<HTMLDivElement | null>(null)
  const loadMoreAbortRef = useRef<AbortController | null>(null)

  const filterKey = `${query}\u0000${artistId ?? ''}\u0000${decade ?? ''}\u0000${
    genre ?? ''
  }`
  const filterKeyRef = useRef(filterKey)
  const hasFilters = Boolean(
    searchInput.trim() || artistId || decade !== null || genre,
  )

  useEffect(() => {
    filterKeyRef.current = filterKey
  }, [filterKey])

  useEffect(() => {
    const trimmed = searchInput.trim()
    if (!trimmed) {
      setQuery('')
      return
    }

    const timer = window.setTimeout(() => setQuery(trimmed), 250)
    return () => window.clearTimeout(timer)
  }, [searchInput])

  const clearFilters = useCallback(() => {
    setSearchInput('')
    setQuery('')
    setArtistId(null)
    setDecade(null)
    setGenre(null)
  }, [])

  useEffect(() => {
    if (!status?.revealed) {
      setRows([])
      setTotal(0)
      setLoading(false)
      return
    }

    loadMoreAbortRef.current?.abort()

    let active = true
    const controller = new AbortController()
    setLoading(true)
    setError(null)
    setAutoLoadPaused(false)
    setRows([])
    setTotal(0)

    api
      .getPublicChart(
        {
          limit: PAGE_SIZE,
          offset: 0,
          q: query,
          artist: artistId,
          decade,
          genre,
        },
        controller.signal,
      )
      .then((data) => {
        if (!active) return
        setRows(data.tracks.map(normalizeOfficial))
        setTotal(data.total)
      })
      .catch((caught: unknown) => {
        if (!active) return
        if (caught instanceof ApiError && caught.status === 403) {
          setError('The standings were sealed again. Refreshing…')
          void refreshPlaylist()
        } else {
          setError(
            caught instanceof ApiError
              ? caught.message
              : 'Could not load the standings',
          )
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [
    status?.revealed,
    status?.playlist_id,
    query,
    artistId,
    decade,
    genre,
    reloadKey,
    refreshPlaylist,
  ])

  useEffect(() => {
    return () => loadMoreAbortRef.current?.abort()
  }, [])

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

    const requestKey = filterKeyRef.current
    const controller = new AbortController()
    loadMoreAbortRef.current = controller
    setLoadingMore(true)
    setError(null)

    try {
      const data = await api.getPublicChart(
        {
          limit: PAGE_SIZE,
          offset: rows.length,
          q: query,
          artist: artistId,
          decade,
          genre,
        },
        controller.signal,
      )
      if (filterKeyRef.current !== requestKey) return
      setRows((current) => [...current, ...data.tracks.map(normalizeOfficial)])
      setTotal(data.total)
      setAutoLoadPaused(false)
    } catch (caught) {
      if (controller.signal.aborted) return
      if (filterKeyRef.current !== requestKey) return

      if (caught instanceof ApiError && caught.status === 403) {
        setError('The standings were sealed again. Refreshing…')
        await refreshPlaylist()
      } else {
        // Do not let the intersection observer retry immediately: during an
        // outage that turns one failed page into a request loop.
        setAutoLoadPaused(true)
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load more songs',
        )
      }
    } finally {
      if (loadMoreAbortRef.current === controller) {
        loadMoreAbortRef.current = null
      }
      setLoadingMore(false)
    }
  }, [
    loading,
    loadingMore,
    rows.length,
    total,
    query,
    artistId,
    decade,
    genre,
    refreshPlaylist,
  ])

  useEffect(() => {
    const node = sentinelRef.current
    if (
      !node ||
      !status?.revealed ||
      loading ||
      loadingMore ||
      autoLoadPaused ||
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
  }, [
    status?.revealed,
    loading,
    loadingMore,
    autoLoadPaused,
    rows.length,
    total,
    loadMore,
  ])

  function refresh() {
    setReloadKey((value) => value + 1)
    void refreshPlaylist()
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

  const podium = !hasFilters && rows.length >= 3 ? rows.slice(0, 3) : []
  const searchPending = searchInput.trim() !== query

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
        <div className="flex min-w-0 flex-wrap items-end justify-between gap-3">
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <span className="pv-label">Public</span>
              <span
                className={`pv-sticker ${
                  status.started_at === null
                    ? 'pv-sticker-butter'
                    : status.voting_paused
                      ? 'pv-sticker-paper'
                      : ''
                }`}
              >
                {status.started_at === null
                  ? 'Scheduled'
                  : status.voting_paused
                    ? 'Voting paused'
                    : 'Now voting'}
              </span>
              <span className="pv-sticker pv-sticker-butter">Reveal open</span>
              <span className="pv-sticker pv-sticker-paper">Official</span>
              <span className="max-w-full break-words pv-label">
                {status.playlist_name}
              </span>
            </div>
            <h1 className="mt-2 break-words font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
              The Standings.
            </h1>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {formatNumber(status.votes_cast)} picks ·{' '}
              {formatNumber(status.total_tracks)} songs ·{' '}
              {formatNumber(status.voters)} voters · {formatNumber(status.duels)} duels
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Link to="/stats" className="btn-ghost">
              The Stats
            </Link>
            <button type="button" onClick={refresh} className="btn-ghost">
              Refresh
            </button>
            {user && (
              <Link to="/your-ranking" className="btn-ghost">
                Your Ranking
              </Link>
            )}
            {user && (
              <Link to="/duel" className="btn">
                Duel
              </Link>
            )}
          </div>
        </div>

        {error && (
          <p
            role="alert"
            className="mt-4 rounded-xl border-2 border-danger bg-danger/5 px-3 py-2 text-sm text-danger"
          >
            {error}
          </p>
        )}
      </header>

      <TrackFilters
        ariaLabel="Find a song in the standings"
        searchLabel="Search the standings"
        search={searchInput}
        onSearchChange={setSearchInput}
        artists={status.artists ?? []}
        artistId={artistId}
        onArtistChange={setArtistId}
        decades={status.decades ?? []}
        decade={decade}
        onDecadeChange={setDecade}
        genres={status.genres ?? []}
        genre={genre}
        onGenreChange={setGenre}
        resultCount={total}
        totalCount={status.total_tracks}
        loading={loading || loadingMore || searchPending}
        onClear={clearFilters}
      />

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
                <p className="pv-label">
                  {hasFilters ? 'Filtered view' : 'Official pressing'}
                </p>
                <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
                  {hasFilters ? 'Matching songs' : 'The full tracklist'}
                </h2>
              </div>
              <span className="pv-label">
                {formatNumber(total)} {hasFilters ? 'matches' : 'songs'}
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
                {rows.map((row) => (
                  <li
                    key={row.id}
                    className="pv-list-row grid grid-cols-[2.5rem_2.5rem_1fr] items-center gap-3 border-b border-line px-3 py-3 last:border-b-0 sm:grid-cols-[2.5rem_2.5rem_1fr_auto]"
                  >
                    <span className="font-serif text-xl leading-none text-muted">
                      {String(row.rank).padStart(2, '0')}
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
                      <span className="font-serif text-lg leading-none text-ink">
                        {Math.round(row.score)}
                      </span>
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
                onClick={() => {
                  setAutoLoadPaused(false)
                  void loadMore()
                }}
                className="btn-ghost mt-4 w-full"
              >
                {autoLoadPaused ? 'Try loading more again' : 'Load more'}
              </button>
            )}

            <p
              role="status"
              aria-live="polite"
              className="mt-4 text-center font-mono text-[9px] uppercase tracking-[0.14em] text-muted"
            >
              Showing {formatNumber(rows.length)} of {formatNumber(total)}{' '}
              {hasFilters ? 'matches' : 'songs'}
              {rows.length >= total
                ? hasFilters
                  ? ' · that’s every match'
                  : ' · that’s the whole tape'
                : ''}
            </p>
          </section>
        </>
      ) : hasFilters ? (
        <section className="pv-card mt-6 p-6 text-center">
          <p className="text-sm text-muted">No songs match these filters.</p>
          <button type="button" onClick={clearFilters} className="btn-ghost mt-3">
            Clear filters
          </button>
        </section>
      ) : (
        <section className="pv-card mt-6 p-6 text-center">
          <p className="text-sm text-muted">The standings are empty.</p>
        </section>
      )}
    </main>
  )
}
