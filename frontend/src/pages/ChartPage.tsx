import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'

import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type { BalancedChart, ChartStatus, PublicChart } from '../lib/types'

const PAGE_SIZE = 50

type Mode = 'official' | 'balanced'

type DisplayRow = {
  id: string
  rank: number | null
  title: string
  artists: string
  albumImageUrl: string | null
  score: number | null
  comparisons: number
  wins: number
  voters: number | null
  noData: boolean
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function ChartPage() {
  const { user } = useAuth()
  const [status, setStatus] = useState<ChartStatus | null>(null)
  const [mode, setMode] = useState<Mode>('official')
  const [chart, setChart] = useState<PublicChart | null>(null)
  const [balanced, setBalanced] = useState<BalancedChart | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [refreshing, setRefreshing] = useState(false)

  useEffect(() => {
    let active = true

    api
      .getChartStatus()
      .then((next) => {
        if (active) setStatus(next)
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load the standings status')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [])

  useEffect(() => {
    if (!status?.revealed) {
      setChart(null)
      setBalanced(null)
      return
    }

    let active = true
    setLoading(true)
    setError(null)

    const request =
      mode === 'official'
        ? api.getPublicChart(offset, PAGE_SIZE, query).then((data) => {
            if (active) setChart(data)
          })
        : api.getBalancedChart(offset, PAGE_SIZE, query).then((data) => {
            if (active) setBalanced(data)
          })

    request
      .catch((caught: unknown) => {
        if (!active) return
        if (caught instanceof ApiError && caught.status === 403) {
          // The admin sealed the standings again while this page was open.
          setStatus((previous) =>
            previous ? { ...previous, revealed: false } : previous,
          )
          setChart(null)
          setBalanced(null)
        } else {
          setError(
            caught instanceof ApiError
              ? caught.message
              : mode === 'official'
                ? 'Could not load the standings'
                : 'Could not load the balanced standings',
          )
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [status?.revealed, offset, query, mode])

  async function refresh() {
    setRefreshing(true)
    setError(null)

    try {
      const next = await api.getChartStatus()
      setStatus(next)
      if (next.revealed) {
        if (mode === 'official') {
          setChart(await api.getPublicChart(offset, PAGE_SIZE, query))
        } else {
          setBalanced(await api.getBalancedChart(offset, PAGE_SIZE, query))
        }
      }
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : mode === 'official'
            ? 'Could not refresh the standings'
            : 'Could not refresh the balanced standings',
      )
    } finally {
      setRefreshing(false)
    }
  }

  function applySearch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setOffset(0)
    setQuery(queryInput.trim())
  }

  function clearSearch() {
    setQueryInput('')
    setQuery('')
    setOffset(0)
  }

  function selectMode(next: Mode) {
    if (next === mode) return

    setMode(next)
    setOffset(0)
    setError(null)
    setLoading(true)
  }

  if (loading && !status) {
    return (
      <>
        <div className="h-24 animate-pulse border-b border-line bg-card" />
        <div className="mt-6 space-y-2">
          {[0, 1, 2, 3, 4].map((index) => (
            <div key={index} className="h-14 animate-pulse border border-line bg-card" />
          ))}
        </div>
      </>
    )
  }

  if (error && !status) {
    return (
      <section className="panel p-6 text-center">
        <p className="label">Unavailable</p>
        <h1 className="mt-3 font-serif text-3xl">The standings could not load.</h1>
        <p className="mt-3 text-sm text-muted">{error}</p>
        <Link to="/" className="btn mt-5 inline-flex">
          Back home
        </Link>
      </section>
    )
  }

  if (!status) return null

  if (!status.playlist_name) {
    return (
      <section className="panel p-6 text-center sm:p-10">
        <p className="label">No standings yet</p>
        <h1 className="mt-3 font-serif text-4xl">The playlist is not in.</h1>
        <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
          The admin has not imported a playlist yet, so there is nothing to
          reveal.
        </p>
        <Link to="/" className="btn mt-6 inline-flex">
          Back home
        </Link>
      </section>
    )
  }

  if (!status.revealed) {
    return (
      <>
        <div className="border-b border-line pb-4">
          <p className="label">Public</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">
            The Standings are sealed.
          </h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs
          </p>
        </div>

        <section className="panel mt-6 p-6 text-center sm:p-10">
          <p className="label">Not revealed</p>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            The admin has not opened The Reveal yet. Picking stays open — the
            standings are being built in private.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
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
      </>
    )
  }

  const active = mode === 'official' ? chart : balanced
  const rows: DisplayRow[] =
    mode === 'official'
      ? (chart?.tracks ?? []).map((entry) => ({
          id: entry.track.id,
          rank: entry.rank,
          title: entry.track.title,
          artists: entry.track.artists,
          albumImageUrl: entry.track.album_image_url,
          score: entry.rating,
          comparisons: entry.comparisons,
          wins: entry.wins,
          voters: null,
          noData: false,
        }))
      : (balanced?.tracks ?? []).map((entry) => ({
          id: entry.track.id,
          rank: entry.rank,
          title: entry.track.title,
          artists: entry.track.artists,
          albumImageUrl: entry.track.album_image_url,
          score: entry.rating,
          comparisons: entry.comparisons,
          wins: entry.wins,
          voters: entry.voters,
          noData: entry.no_data,
        }))

  const first = active && active.total > 0 ? active.offset + 1 : 0
  const last = active ? Math.min(active.offset + active.limit, active.total) : 0

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-4">
        <div>
          <p className="label">Public</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">The Standings.</h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs
          </p>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
            Live standings · updates as picks come in
          </p>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => void refresh()}
            disabled={refreshing}
            className="btn-ghost"
          >
            {refreshing ? 'Refreshing…' : 'Refresh'}
          </button>
          {user && (
            <Link to="/duel" className="btn">
              Duel
            </Link>
          )}
        </div>
      </div>

      <div className="mt-5 flex flex-wrap items-center gap-2 border-b border-line pb-3">
        <button
          type="button"
          onClick={() => selectMode('official')}
          className={mode === 'official' ? 'btn' : 'btn-ghost'}
          aria-pressed={mode === 'official'}
        >
          Official
        </button>
        <button
          type="button"
          onClick={() => selectMode('balanced')}
          className={mode === 'balanced' ? 'btn' : 'btn-ghost'}
          aria-pressed={mode === 'balanced'}
        >
          Balanced
        </button>
        <span className="ml-auto font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
          {mode === 'official'
            ? 'Every pick counts once'
            : 'Volume-adjusted · confidence-weighted'}
        </span>
      </div>

      {mode === 'balanced' && (
        <p className="mt-3 max-w-2xl text-sm leading-relaxed text-muted">
          Influence grows with the square root of each voter&apos;s pick count by
          default, so heavy voters cannot run away with the list. Personal scores
          are shrunk toward neutral until a voter has compared a song enough
          times.
        </p>
      )}

      <form onSubmit={applySearch} className="mt-4 flex flex-col gap-2 sm:flex-row">
        <input
          type="search"
          value={queryInput}
          onChange={(event) => setQueryInput(event.target.value)}
          placeholder="Search songs, artists or albums…"
          className="field flex-1"
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
        <p className="mt-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-sm text-accent">
          {error}
        </p>
      )}

      {loading ? (
        <div className="mt-4 space-y-2">
          {[0, 1, 2, 3, 4].map((index) => (
            <div key={index} className="h-14 animate-pulse border border-line bg-card" />
          ))}
        </div>
      ) : rows.length > 0 ? (
        <>
          <div className="mt-4 border border-line bg-card">
            <div className="hidden grid-cols-[2.5rem_3rem_1fr_auto] gap-3 border-b border-line px-4 py-2.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted sm:grid">
              <span>#</span>
              <span />
              <span>Song</span>
              <span className="text-right">
                {mode === 'official'
                  ? 'Wins · Duels · Score'
                  : 'Wins · Duels · Voters · Score'}
              </span>
            </div>

            <ol>
              {rows.map((row) => (
                <li
                  key={row.id}
                  className="grid grid-cols-[2.5rem_3rem_1fr] items-center gap-3 border-b border-line px-4 py-3 last:border-b-0 sm:grid-cols-[2.5rem_3rem_1fr_auto]"
                >
                  <span className="font-serif text-xl leading-none text-muted">
                    {row.rank ? String(row.rank).padStart(2, '0') : '—'}
                  </span>

                  {row.albumImageUrl ? (
                    <img
                      src={row.albumImageUrl}
                      alt=""
                      className="size-12 border border-line object-cover"
                    />
                  ) : (
                    <div className="grid size-12 place-items-center border border-line bg-paper font-serif text-lg text-muted/40">
                      ♪
                    </div>
                  )}

                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium leading-tight">
                      {row.title}
                    </p>
                    <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                      {row.artists}
                    </p>
                  </div>

                  <div className="col-span-3 flex flex-wrap items-center justify-end gap-3 font-mono text-[10px] uppercase tracking-[0.1em] text-muted sm:col-span-1 sm:gap-4">
                    <span>
                      <span className="text-ink">{row.wins}</span> W
                    </span>
                    <span className="hidden sm:inline">
                      <span className="text-ink">{row.comparisons}</span> D
                    </span>
                    {mode === 'balanced' && row.voters !== null && (
                      <span className="hidden sm:inline">
                        <span className="text-ink">{row.voters}</span> V
                      </span>
                    )}
                    {row.noData || row.score === null ? (
                      <span className="text-muted/70">No data</span>
                    ) : (
                      <span className="text-ink">{Math.round(row.score)}</span>
                    )}
                  </div>
                </li>
              ))}
            </ol>
          </div>

          <div className="mt-5 flex items-center justify-between gap-3">
            <button
              type="button"
              onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
              disabled={offset === 0}
              className="btn-ghost"
            >
              Previous
            </button>
            <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {first}–{last} of {formatNumber(active?.total ?? 0)}
            </span>
            <button
              type="button"
              onClick={() => setOffset(offset + PAGE_SIZE)}
              disabled={!active || last >= active.total}
              className="btn-ghost"
            >
              Next
            </button>
          </div>
        </>
      ) : (
        <section className="panel mt-4 p-6 text-center">
          <p className="text-sm text-muted">
            {query
              ? `No songs match “${query}”.`
              : mode === 'balanced'
                ? 'The playlist is empty.'
                : 'The standings are empty.'}
          </p>
        </section>
      )}
    </>
  )
}
