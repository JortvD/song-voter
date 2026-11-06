import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'

import { RequireAuth } from '../components/RequireAuth'
import { TrackFilters } from '../components/TrackFilters'
import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type { PersonalRanking, PersonalRankedTrack } from '../lib/types'

const PAGE_SIZE = 50

function formatNumber(value: number) {
  return value.toLocaleString()
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

function TrackRow({
  entry,
  highlight,
}: {
  entry: PersonalRankedTrack
  highlight: boolean
}) {
  return (
    <li
      className={`pv-list-row grid grid-cols-[2.5rem_2.5rem_1fr_auto] items-center gap-3 border-b border-line px-3 py-3 last:border-b-0 ${
        highlight ? 'bg-[var(--playlist-accent-tint)]' : ''
      }`}
    >
      <span className="font-serif text-xl leading-none text-muted">
        {String(entry.rank).padStart(2, '0')}
      </span>

      <Cover image={entry.track.album_image_url} title={entry.track.title} />

      <div className="min-w-0">
        <p className="truncate text-sm font-medium leading-tight">
          {entry.track.title}
        </p>
        <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {entry.track.artists}
        </p>
      </div>

      <div className="flex items-center justify-end gap-3 font-mono text-[9px] uppercase tracking-[0.1em] text-muted sm:gap-4">
        <span>
          <span className="text-ink">{entry.wins}</span> W
        </span>
        <span className="hidden sm:inline">
          <span className="text-ink">{entry.comparisons}</span> D
        </span>
        <span className="font-serif text-lg leading-none text-ink">
          {Math.round(entry.rating)}
        </span>
      </div>
    </li>
  )
}

function RankingScreen() {
  const { user } = useAuth()
  const [ranking, setRanking] = useState<PersonalRanking | null>(null)
  const [loadedFilterKey, setLoadedFilterKey] = useState('')
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [reloadKey, setReloadKey] = useState(0)
  const [searchInput, setSearchInput] = useState('')
  const [query, setQuery] = useState('')
  const [artistId, setArtistId] = useState<string | null>(null)
  const [decade, setDecade] = useState<number | null>(null)
  const [genre, setGenre] = useState<string | null>(null)
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

  const reload = useCallback(() => {
    setReloadKey((value) => value + 1)
  }, [])

  useEffect(() => {
    loadMoreAbortRef.current?.abort()

    let active = true
    const controller = new AbortController()
    const requestKey = filterKey
    setLoading(true)
    setError(null)

    api
      .getPersonalRanking(
        0,
        PAGE_SIZE,
        {
          q: query,
          artist: artistId,
          decade,
          genre,
        },
        controller.signal,
      )
      .then((data) => {
        if (!active) return
        setRanking(data)
        setLoadedFilterKey(requestKey)
      })
      .catch((caught) => {
        if (!active || controller.signal.aborted) return
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load your ranking',
        )
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [query, artistId, decade, genre, filterKey, reloadKey])

  useEffect(() => {
    return () => loadMoreAbortRef.current?.abort()
  }, [])

  const loadMore = useCallback(async () => {
    if (!ranking || loading || loadingMore || ranking.tracks.length >= ranking.total) {
      return
    }

    const requestKey = filterKeyRef.current
    const controller = new AbortController()
    loadMoreAbortRef.current = controller
    setLoadingMore(true)
    setError(null)

    try {
      const next = await api.getPersonalRanking(
        ranking.tracks.length,
        PAGE_SIZE,
        {
          q: query,
          artist: artistId,
          decade,
          genre,
        },
        controller.signal,
      )
      if (filterKeyRef.current !== requestKey) return

      setRanking((current) =>
        current
          ? {
              ...current,
              total: next.total,
              ranked_total: next.ranked_total,
              top_score: next.top_score,
              votes: next.votes,
              decades: next.decades,
              genres: next.genres,
              artists: next.artists,
              tracks: [...current.tracks, ...next.tracks],
            }
          : next,
      )
    } catch (caught) {
      if (controller.signal.aborted) return
      if (filterKeyRef.current !== requestKey) return
      setError(
        caught instanceof ApiError ? caught.message : 'Could not load more songs',
      )
    } finally {
      if (loadMoreAbortRef.current === controller) {
        loadMoreAbortRef.current = null
      }
      setLoadingMore(false)
    }
  }, [ranking, loading, loadingMore, query, artistId, decade, genre])

  if (loading && !ranking) {
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

  if (error && !ranking) {
    const missingPlaylist = error.toLowerCase().includes('playlist')

    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <section className="pv-card p-6 text-center sm:p-10">
          <span className="pv-sticker pv-sticker-paper">
            {missingPlaylist ? 'No tape yet' : 'Unavailable'}
          </span>
          <h1 className="mt-4 font-serif text-4xl leading-tight tracking-[-0.03em]">
            {missingPlaylist
              ? 'Your Ranking needs a tape.'
              : 'Your Ranking could not load.'}
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            {missingPlaylist
              ? 'The admin has not imported a playlist yet. Once there is a tape, your picks will appear here.'
              : error}
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            {missingPlaylist && user?.is_admin && (
              <Link to="/admin" className="btn">
                Import a tape
              </Link>
            )}
            <button type="button" onClick={reload} className="btn-ghost">
              Try again
            </button>
            <Link to="/" className="btn-ghost">
              Back home
            </Link>
          </div>
        </section>
      </main>
    )
  }

  if (!ranking) return null

  if (ranking.votes === 0) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <section className="pv-card p-6 text-center sm:p-10">
          <span className="pv-sticker pv-sticker-butter">Blank tape</span>
          <h1 className="mt-4 font-serif text-4xl leading-tight tracking-[-0.03em]">
            Your Ranking starts with one pick.
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            Pick a few duels and the songs you&apos;ve heard will appear here.
            This list is private to you.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <Link to="/duel" className="btn">
              Start a duel
            </Link>
            <Link to="/" className="btn-ghost">
              How it works
            </Link>
          </div>
        </section>
      </main>
    )
  }

  const dataMatchesFilters = loadedFilterKey === filterKey
  const top =
    !hasFilters && dataMatchesFilters ? ranking.tracks[0] : null
  const hasMore = ranking.tracks.length < ranking.total
  const searchPending = searchInput.trim() !== query

  return (
    <main className="pv-rise mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      <header className="border-b-2 border-ink/15 pb-4">
        <div className="flex min-w-0 flex-wrap items-end justify-between gap-3">
          <div className="min-w-0">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <span className="pv-label">Private</span>
              <span className="pv-sticker">Your side</span>
              <span className="pv-sticker pv-sticker-paper">Your tape</span>
              <span className="max-w-full break-words pv-label">
                {ranking.playlist_name}
              </span>
            </div>
            <h1 className="mt-2 break-words font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
              Your Ranking.
            </h1>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {formatNumber(ranking.votes)} picks ·{' '}
              {formatNumber(ranking.ranked_total)} songs ranked · Private to you
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Link to="/duel" className="btn">
              Next duel
            </Link>
            <Link to="/standings" className="btn-ghost">
              The Standings
            </Link>
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
        ariaLabel="Find a song in your ranking"
        searchLabel="Search your ranking"
        searchPlaceholder="Find a song, artist or album"
        search={searchInput}
        onSearchChange={setSearchInput}
        artists={ranking.artists ?? []}
        artistId={artistId}
        onArtistChange={setArtistId}
        decades={ranking.decades ?? []}
        decade={decade}
        onDecadeChange={setDecade}
        genres={ranking.genres ?? []}
        genre={genre}
        onGenreChange={setGenre}
        resultCount={ranking.total}
        totalCount={ranking.ranked_total}
        loading={loading || loadingMore || searchPending}
        onClear={clearFilters}
      />

      <section className="mt-6 grid gap-3 sm:grid-cols-3">
        <StatPill label="Your picks" value={formatNumber(ranking.votes)} />
        <StatPill
          label="Songs ranked"
          value={formatNumber(ranking.ranked_total)}
        />
        <StatPill
          label="Your top score"
          value={
            ranking.top_score === null
              ? '—'
              : Math.round(ranking.top_score).toString()
          }
        />
      </section>

      {top && (
        <section className="pv-card mt-4 flex flex-col gap-4 p-4 sm:flex-row sm:items-center sm:p-5">
          <div className="flex min-w-0 flex-1 items-center gap-4">
            <Cover
              image={top.track.album_image_url}
              title={top.track.title}
              size="size-20"
            />
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <span className="pv-sticker pv-sticker-butter">Your #1</span>
                <span className="pv-label">Currently on top</span>
              </div>
              <h2 className="mt-2 truncate font-serif text-2xl leading-tight tracking-[-0.02em]">
                {top.track.title}
              </h2>
              <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                {top.track.artists}
              </p>
            </div>
          </div>
          <div className="grid w-full grid-cols-3 gap-2 sm:w-auto">
            <StatPill label="Wins" value={formatNumber(top.wins)} />
            <StatPill label="Duels" value={formatNumber(top.comparisons)} />
            <StatPill label="Score" value={Math.round(top.rating).toString()} />
          </div>
        </section>
      )}

      <section className="mt-6" aria-busy={loading || loadingMore}>
        <div className="flex flex-wrap items-end justify-between gap-3 border-b-2 border-ink/15 pb-2">
          <div>
            <p className="pv-label">
              {hasFilters ? 'Filtered view' : 'Your mixtape'}
            </p>
            <h2 className="mt-1 font-serif text-2xl tracking-[-0.02em]">
              {hasFilters ? 'Matching songs' : 'The tracklist'}
            </h2>
          </div>
          <span className="pv-label">
            {loading
              ? 'Searching…'
              : hasFilters
                ? `${formatNumber(ranking.total)} matches`
                : `${formatNumber(ranking.tracks.length)} of ${formatNumber(
                    ranking.ranked_total,
                  )}`}
          </span>
        </div>

        {loading ? (
          <div className="mt-3 space-y-2">
            {[0, 1, 2, 3, 4].map((index) => (
              <div
                key={index}
                className="h-16 animate-pulse rounded-xl border-2 border-line bg-card"
              />
            ))}
          </div>
        ) : ranking.total === 0 ? (
          <section className="pv-card mt-3 p-6 text-center">
            <p className="text-sm text-muted">
              {hasFilters
                ? 'No songs match these filters.'
                : 'No songs ranked yet.'}
            </p>
            {hasFilters ? (
              <button
                type="button"
                onClick={clearFilters}
                className="btn-ghost mt-3"
              >
                Clear filters
              </button>
            ) : (
              <Link to="/duel" className="btn mt-3">
                Start a duel
              </Link>
            )}
          </section>
        ) : (
          <>
            <div className="mt-3 overflow-hidden rounded-2xl border-2 border-ink bg-card">
              <div className="hidden grid-cols-[2.5rem_2.5rem_1fr_auto] items-center gap-3 border-b-2 border-ink px-3 py-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted sm:grid">
                <span>#</span>
                <span />
                <span>Song</span>
                <span className="text-right">Wins · Duels · Score</span>
              </div>

              <ol>
                {ranking.tracks.map((entry) => (
                  <TrackRow
                    key={entry.track.id}
                    entry={entry}
                    highlight={!hasFilters && entry.rank === 1}
                  />
                ))}
              </ol>
            </div>

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

            {hasMore && (
              <button
                type="button"
                onClick={() => void loadMore()}
                disabled={loadingMore}
                className="btn-ghost mt-4 w-full"
              >
                {loadingMore ? 'Loading more…' : 'Load more'}
              </button>
            )}

            <p
              role="status"
              aria-live="polite"
              className="mt-4 text-center font-mono text-[9px] uppercase tracking-[0.14em] text-muted"
            >
              Showing {formatNumber(ranking.tracks.length)} of{' '}
              {formatNumber(ranking.total)}{' '}
              {hasFilters ? 'matches' : 'songs'}
              {hasMore
                ? ''
                : hasFilters
                  ? ' · that’s every match'
                  : ' · that’s your whole tape'}
            </p>
          </>
        )}
      </section>
    </main>
  )
}

export function RankingPage() {
  return (
    <RequireAuth>
      <RankingScreen />
    </RequireAuth>
  )
}
