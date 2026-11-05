import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'

import { useAuth } from '../context/AuthContext'
import { ApiError, api } from '../lib/api'
import type { ChartStatus, PublicChart } from '../lib/types'

const PAGE_SIZE = 50

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function ChartPage() {
  const { user } = useAuth()
  const [status, setStatus] = useState<ChartStatus | null>(null)
  const [chart, setChart] = useState<PublicChart | null>(null)
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
          setError(caught instanceof ApiError ? caught.message : 'Could not load the chart status')
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
      return
    }

    let active = true
    setLoading(true)
    setError(null)

    api
      .getPublicChart(offset, PAGE_SIZE, query)
      .then((data) => {
        if (active) setChart(data)
      })
      .catch((caught: unknown) => {
        if (!active) return
        if (caught instanceof ApiError && caught.status === 403) {
          // The admin hid the chart again while this page was open.
          setStatus((previous) =>
            previous ? { ...previous, revealed: false } : previous,
          )
          setChart(null)
        } else {
          setError(caught instanceof ApiError ? caught.message : 'Could not load the chart')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [status?.revealed, offset, query])

  async function refresh() {
    setRefreshing(true)
    setError(null)

    try {
      const next = await api.getChartStatus()
      setStatus(next)
      if (next.revealed) {
        setChart(await api.getPublicChart(offset, PAGE_SIZE, query))
      }
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not refresh the chart')
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
        <h1 className="mt-3 font-serif text-3xl">The chart could not load.</h1>
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
        <p className="label">No chart yet</p>
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
          <p className="label">Shared chart</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">
            The chart is sealed.
          </h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs
          </p>
        </div>

        <section className="panel mt-6 p-6 text-center sm:p-10">
          <p className="label">Not revealed</p>
          <p className="mx-auto mt-3 max-w-md text-sm leading-relaxed text-muted">
            The admin has not revealed the shared Elo ranking yet. Voting stays
            open — the list is being built in private.
          </p>
          <div className="mt-6 flex flex-wrap justify-center gap-3">
            <Link to={user ? '/vote' : '/#auth'} className="btn">
              {user ? 'Keep voting' : 'Sign in to vote'}
            </Link>
            {user && (
              <Link to="/ranking" className="btn-ghost">
                Your personal chart
              </Link>
            )}
          </div>
        </section>
      </>
    )
  }

  const first = chart && chart.total > 0 ? chart.offset + 1 : 0
  const last = chart ? Math.min(chart.offset + chart.limit, chart.total) : 0

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-4">
        <div>
          <p className="label">Shared chart</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">The chart.</h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {status.playlist_name} · {formatNumber(status.votes_cast)} picks ·{' '}
            {formatNumber(status.total_tracks)} songs
          </p>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted/70">
            Live Elo ranking · updates as votes come in
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
            <Link to="/vote" className="btn">
              Vote
            </Link>
          )}
        </div>
      </div>

      <form onSubmit={applySearch} className="mt-4 flex flex-col gap-2 sm:flex-row">
        <input
          type="search"
          value={queryInput}
          onChange={(event) => setQueryInput(event.target.value)}
          placeholder="Search songs, artists or albums…"
          className="field flex-1"
          aria-label="Search the chart"
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
      ) : chart && chart.tracks.length > 0 ? (
        <>
          <div className="mt-4 border border-line bg-card">
            <div className="hidden grid-cols-[2.5rem_3rem_1fr_auto] gap-3 border-b border-line px-4 py-2.5 font-mono text-[9px] uppercase tracking-[0.14em] text-muted sm:grid">
              <span>#</span>
              <span />
              <span>Song</span>
              <span className="text-right">Wins · Picks · Elo</span>
            </div>

            <ol>
              {chart.tracks.map((entry) => (
                <li
                  key={entry.track.id}
                  className="grid grid-cols-[2.5rem_3rem_1fr] items-center gap-3 border-b border-line px-4 py-3 last:border-b-0 sm:grid-cols-[2.5rem_3rem_1fr_auto]"
                >
                  <span className="font-serif text-xl leading-none text-muted">
                    {String(entry.rank).padStart(2, '0')}
                  </span>

                  {entry.track.album_image_url ? (
                    <img
                      src={entry.track.album_image_url}
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
                      {entry.track.title}
                    </p>
                    <p className="mt-0.5 truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                      {entry.track.artists}
                    </p>
                  </div>

                  <div className="col-span-3 flex items-center justify-end gap-4 font-mono text-[10px] uppercase tracking-[0.1em] text-muted sm:col-span-1">
                    <span>
                      <span className="text-ink">{entry.wins}</span> W
                    </span>
                    <span className="hidden sm:inline">
                      <span className="text-ink">{entry.comparisons}</span> P
                    </span>
                    <span className="text-ink">{Math.round(entry.rating)}</span>
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
              {first}–{last} of {formatNumber(chart.total)}
            </span>
            <button
              type="button"
              onClick={() => setOffset(offset + PAGE_SIZE)}
              disabled={last >= chart.total}
              className="btn-ghost"
            >
              Next
            </button>
          </div>
        </>
      ) : (
        <section className="panel mt-4 p-6 text-center">
          <p className="text-sm text-muted">
            {query ? `No songs match “${query}”.` : 'The chart is empty.'}
          </p>
        </section>
      )}
    </>
  )
}
