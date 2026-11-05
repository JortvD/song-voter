import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type { BalancedChart, PublicChart } from '../lib/types'

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
  voters: number
  weight: number
  noData: boolean
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminRankingTable() {
  const [mode, setMode] = useState<Mode>('official')
  const [data, setData] = useState<PublicChart | BalancedChart | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)
    setData(null)

    const request =
      mode === 'official'
        ? api.getAdminRanking(offset, PAGE_SIZE, query)
        : api.getAdminBalanced(offset, PAGE_SIZE, query)

    request
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught: unknown) => {
        if (!active) return
        if (caught instanceof ApiError && caught.status === 404) {
          setData(null)
          setError(null)
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
  }, [offset, query, mode])

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

  const rows: DisplayRow[] =
    mode === 'official'
      ? ((data as PublicChart | null)?.tracks ?? []).map((entry) => ({
          id: entry.track.id,
          rank: entry.rank,
          title: entry.track.title,
          artists: entry.track.artists,
          albumImageUrl: entry.track.album_image_url,
          score: entry.rating,
          comparisons: entry.comparisons,
          wins: entry.wins,
          voters: 0,
          weight: 0,
          noData: false,
        }))
      : ((data as BalancedChart | null)?.tracks ?? []).map((entry) => ({
          id: entry.track.id,
          rank: entry.rank,
          title: entry.track.title,
          artists: entry.track.artists,
          albumImageUrl: entry.track.album_image_url,
          score: entry.rating,
          comparisons: entry.comparisons,
          wins: entry.wins,
          voters: entry.voters,
          weight: entry.weight,
          noData: entry.no_data,
        }))

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0

  return (
    <section className="panel">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line px-5 py-3">
        <div>
          <h2 className="label">Standings</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {data ? `${formatNumber(data.total)} songs` : 'Loading…'}
          </p>
        </div>

        <form onSubmit={applySearch} className="flex flex-wrap gap-2">
          <input
            type="search"
            value={queryInput}
            onChange={(event) => setQueryInput(event.target.value)}
            placeholder="Search the standings"
            className="field w-52 py-1.5 text-xs"
            aria-label="Search the standings"
          />
          <button type="submit" className="btn-ghost">
            Search
          </button>
          {query && (
            <button type="button" onClick={clearSearch} className="btn-ghost">
              Clear
            </button>
          )}
        </form>
      </div>

      <div className="flex flex-wrap items-center gap-2 border-b border-line px-5 py-2">
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
          {mode === 'official' ? 'Every pick counts once' : 'Volume-adjusted'}
        </span>
      </div>

      <div className="p-5">
        {error && (
          <p className="mb-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
            {error}
          </p>
        )}

        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2, 3, 4].map((index) => (
              <div key={index} className="h-10 animate-pulse border border-line bg-paper" />
            ))}
          </div>
        ) : rows.length > 0 ? (
          <>
            <div className="overflow-x-auto">
              <table
                className={`w-full text-left text-sm ${
                  mode === 'official' ? 'min-w-[620px]' : 'min-w-[760px]'
                }`}
              >
                <thead>
                  <tr className="border-b border-line font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    <th className="w-10 pb-2 font-medium">#</th>
                    <th className="pb-2 font-medium">Song</th>
                    <th className="pb-2 text-right font-medium">Score</th>
                    {mode === 'balanced' && (
                      <>
                        <th className="pb-2 text-right font-medium">Voters</th>
                        <th className="pb-2 text-right font-medium">Weight</th>
                      </>
                    )}
                    <th className="pb-2 text-right font-medium">Duels</th>
                    <th className="pb-2 text-right font-medium">Wins</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr key={row.id} className="border-b border-line last:border-b-0">
                      <td className="py-2.5 font-serif text-lg text-muted">
                        {row.rank ? String(row.rank).padStart(2, '0') : '—'}
                      </td>
                      <td className="py-2.5">
                        <span className="flex items-center gap-2.5">
                          {row.albumImageUrl ? (
                            <img
                              src={row.albumImageUrl}
                              alt=""
                              className="size-8 border border-line object-cover"
                            />
                          ) : (
                            <span className="grid size-8 place-items-center border border-line bg-paper font-serif text-sm text-muted/40">
                              ♪
                            </span>
                          )}
                          <span className="min-w-0">
                            <span className="block truncate font-medium leading-tight">
                              {row.title}
                            </span>
                            <span className="block truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                              {row.artists}
                            </span>
                          </span>
                        </span>
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {row.noData || row.score === null ? (
                          <span className="text-muted/70">No data</span>
                        ) : (
                          Math.round(row.score)
                        )}
                      </td>
                      {mode === 'balanced' && (
                        <>
                          <td className="py-2.5 text-right font-mono text-xs">
                            {formatNumber(row.voters)}
                          </td>
                          <td className="py-2.5 text-right font-mono text-xs">
                            {row.weight.toFixed(2)}
                          </td>
                        </>
                      )}
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(row.comparisons)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(row.wins)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div className="mt-4 flex items-center justify-between gap-3">
              <button
                type="button"
                onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                disabled={offset === 0}
                className="btn-ghost"
              >
                Previous
              </button>
              <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                {first}–{last} of {formatNumber(data?.total ?? 0)}
              </span>
              <button
                type="button"
                onClick={() => setOffset(offset + PAGE_SIZE)}
                disabled={!data || last >= data.total}
                className="btn-ghost"
              >
                Next
              </button>
            </div>
          </>
        ) : (
          <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
            {query ? `No songs match “${query}”.` : 'No tracks in this playlist.'}
          </p>
        )}
      </div>
    </section>
  )
}
