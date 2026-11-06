import { useCallback, useEffect, useRef, useState } from 'react'
import { Link } from 'react-router-dom'

import { HistoryDuelCard } from '../components/duel/HistoryDuelCard'
import { RequireAuth } from '../components/RequireAuth'
import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type {
  DuelHistoryItem,
  DuelHistoryPage,
  DuelHistoryStatus,
  MatchupTrack,
} from '../lib/types'

const PAGE_SIZE = 50

const FILTERS: { key: DuelHistoryStatus; label: string }[] = [
  { key: 'all', label: 'All' },
  { key: 'picked', label: 'Picked' },
  { key: 'skipped', label: 'No pick' },
]

function countForFilter(page: DuelHistoryPage | null, filter: DuelHistoryStatus) {
  if (!page) return ''
  if (filter === 'picked') return page.picked.toLocaleString()
  if (filter === 'skipped') return page.skipped.toLocaleString()
  return page.total.toLocaleString()
}

function HistorySkeleton() {
  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      <div className="h-28 animate-pulse rounded-2xl border-2 border-line bg-card" />
      <div className="mt-6 grid gap-3 md:grid-cols-2">
        {[0, 1, 2, 3].map((index) => (
          <div
            key={index}
            className="h-36 animate-pulse rounded-2xl border-2 border-line bg-card"
          />
        ))}
      </div>
    </main>
  )
}

function DuelHistoryScreen() {
  const { user } = useAuth()
  const [status, setStatus] = useState<DuelHistoryStatus>('all')
  const [page, setPage] = useState<DuelHistoryPage | null>(null)
  const [duels, setDuels] = useState<DuelHistoryItem[]>([])
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [playingTrackId, setPlayingTrackId] = useState<string | null>(null)

  const audioRef = useRef<HTMLAudioElement | null>(null)
  const loadMoreAbortRef = useRef<AbortController | null>(null)

  const stopPreview = useCallback(() => {
    audioRef.current?.pause()
    setPlayingTrackId(null)
  }, [])

  const load = useCallback(
    async (nextStatus: DuelHistoryStatus, signal?: AbortSignal) => {
      setLoading(true)
      setError(null)
      stopPreview()

      try {
        const result = await api.getDuelHistory(0, PAGE_SIZE, nextStatus, signal)
        setPage(result)
        setDuels(result.duels)
      } catch (caught) {
        if (signal?.aborted) return
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load your duels',
        )
        setPage(null)
        setDuels([])
      } finally {
        if (!signal?.aborted) setLoading(false)
      }
    },
    [stopPreview],
  )

  useEffect(() => {
    if (!user) return
    const controller = new AbortController()
    void load(status, controller.signal)
    return () => controller.abort()
  }, [user, status, load])

  useEffect(() => {
    return () => {
      audioRef.current?.pause()
      loadMoreAbortRef.current?.abort()
    }
  }, [])

  const loadMore = useCallback(async () => {
    if (!page || loadingMore || duels.length >= page.filtered_total) return

    const controller = new AbortController()
    loadMoreAbortRef.current = controller
    setLoadingMore(true)
    setError(null)
    try {
      const next = await api.getDuelHistory(
        duels.length,
        PAGE_SIZE,
        status,
        controller.signal,
      )
      setPage((current) =>
        current
          ? {
              ...current,
              total: next.total,
              picked: next.picked,
              skipped: next.skipped,
              filtered_total: next.filtered_total,
            }
          : next,
      )
      setDuels((current) => [...current, ...next.duels])
    } catch (caught) {
      if (controller.signal.aborted) return
      setError(
        caught instanceof ApiError ? caught.message : 'Could not load more duels',
      )
    } finally {
      if (loadMoreAbortRef.current === controller) {
        loadMoreAbortRef.current = null
      }
      setLoadingMore(false)
    }
  }, [page, loadingMore, duels.length, status])

  const togglePreview = useCallback(
    (track: MatchupTrack) => {
      const audio = audioRef.current
      if (!audio || !track.preview_url) return

      if (playingTrackId === track.id) {
        audio.pause()
        setPlayingTrackId(null)
        return
      }

      audio.pause()
      audio.src = track.preview_url
      audio.load()
      void audio
        .play()
        .then(() => setPlayingTrackId(track.id))
        .catch(() => setPlayingTrackId(null))
    },
    [playingTrackId],
  )

  if (loading && !page) return <HistorySkeleton />

  if (error && !page) {
    return (
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-8 sm:px-6 sm:py-10 lg:px-8">
        <section className="pv-card p-6 text-center sm:p-10">
          <span className="pv-sticker pv-sticker-paper">Unavailable</span>
          <h1 className="mt-4 font-serif text-4xl leading-tight tracking-[-0.03em]">
            Your duels could not load.
          </h1>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            {error}
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <button type="button" onClick={() => void load(status)} className="btn-ghost">
              Try again
            </button>
            <Link to="/duel" className="btn">
              Back to duel
            </Link>
          </div>
        </section>
      </main>
    )
  }

  if (!page) return null

  const hasMore = duels.length < page.filtered_total
  const noDuels = duels.length === 0

  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6 sm:py-8 lg:px-8">
      <header className="border-b-2 border-ink/15 pb-4">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <p className="pv-label">Duel history · {page.playlist_name}</p>
            <h1 className="mt-2 font-serif text-4xl leading-none tracking-[-0.04em] sm:text-5xl">
              Your duels.
            </h1>
            <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {page.total.toLocaleString()} duels · {page.picked.toLocaleString()}{' '}
              picked · {page.skipped.toLocaleString()} no pick
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Link to="/duel" className="btn">
              Back to duel
            </Link>
            <Link to="/your-ranking" className="btn-ghost">
              Your Ranking
            </Link>
          </div>
        </div>
      </header>

      <div className="mt-5 flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap gap-1.5" role="group" aria-label="Filter duels">
          {FILTERS.map((filter) => (
            <button
              key={filter.key}
              type="button"
              onClick={() => setStatus(filter.key)}
              disabled={loading}
              aria-pressed={status === filter.key}
              className={`pv-tab ${status === filter.key ? 'pv-tab-active' : ''}`}
            >
              {filter.label}
              <span className="ml-1">
                {countForFilter(page, filter.key)}
              </span>
            </button>
          ))}
        </div>

        <p className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
          Tap a cover to preview
        </p>
      </div>

      {error && (
        <p role="alert" className="pv-duel-error mt-4">
          {error}
        </p>
      )}

      {noDuels ? (
        <section className="pv-card mt-5 p-6 text-center sm:p-10">
          <span className="pv-sticker pv-sticker-paper">Blank tape</span>
          <h2 className="mt-4 font-serif text-3xl leading-tight tracking-[-0.03em]">
            {status === 'all'
              ? 'No duels served yet.'
              : `No ${status === 'picked' ? 'picked' : 'skipped'} duels.`}
          </h2>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            {status === 'all'
              ? 'Your served duels will appear here, including the ones you leave without picking.'
              : 'Try another filter or head back to the duel screen.'}
          </p>
          <Link to="/duel" className="btn mt-5">
            Start a duel
          </Link>
        </section>
      ) : (
        <>
          <div className="mt-5 grid gap-3 md:grid-cols-2">
            {duels.map((duel) => (
              <HistoryDuelCard
                key={`${duel.track_a.id}:${duel.track_b.id}`}
                duel={duel}
                playingTrackId={playingTrackId}
                onTogglePreview={togglePreview}
              />
            ))}
          </div>

          {hasMore && (
            <div className="mt-6 flex justify-center">
              <button
                type="button"
                onClick={() => void loadMore()}
                disabled={loadingMore || loading}
                className="btn-ghost"
              >
                {loadingMore ? 'Loading…' : 'Load more'}
              </button>
            </div>
          )}
        </>
      )}

      <div className="mt-8 flex flex-wrap justify-center gap-3">
        <Link to="/duel" className="btn">
          Back to duel
        </Link>
      </div>

      <audio
        ref={audioRef}
        preload="metadata"
        className="hidden"
        aria-label="Duel history preview"
        onEnded={() => setPlayingTrackId(null)}
      />
    </main>
  )
}

export function DuelHistoryPage() {
  return (
    <RequireAuth>
      <DuelHistoryScreen />
    </RequireAuth>
  )
}
