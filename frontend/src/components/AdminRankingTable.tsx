import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type { PublicChart } from '../lib/types'

const PAGE_SIZE = 50

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminRankingTable() {
  const [data, setData] = useState<PublicChart | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)
    setData(null)

    api
      .getAdminRanking(offset, PAGE_SIZE, query)
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
              : 'Could not load the standings',
          )
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [offset, query, refreshKey])

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

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0

  return (
    <section className="panel">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line px-5 py-3">
        <div>
          <h2 className="label">Standings</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {data
              ? `${formatNumber(data.total)} songs`
              : error
                ? 'Unavailable'
                : 'Loading…'}
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

      <div className="p-5">
        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2, 3, 4].map((index) => (
              <div key={index} className="h-10 animate-pulse border border-line bg-paper" />
            ))}
          </div>
        ) : error ? (
          <div role="alert" className="border border-danger/40 bg-danger/5 px-4 py-6 text-center">
            <p className="text-xs text-danger">{error}</p>
            <button
              type="button"
              onClick={() => setRefreshKey((value) => value + 1)}
              className="btn-ghost mt-3"
            >
              Try again
            </button>
          </div>
        ) : data && data.tracks.length > 0 ? (
          <>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[620px] text-left text-sm">
                <thead>
                  <tr className="border-b border-line font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    <th className="w-10 pb-2 font-medium">#</th>
                    <th className="pb-2 font-medium">Song</th>
                    <th className="pb-2 text-right font-medium">Score</th>
                    <th className="pb-2 text-right font-medium">Duels</th>
                    <th className="pb-2 text-right font-medium">Wins</th>
                  </tr>
                </thead>
                <tbody>
                  {data.tracks.map((entry) => (
                    <tr key={entry.track.id} className="border-b border-line last:border-b-0">
                      <td className="py-2.5 font-serif text-lg text-muted">
                        {String(entry.rank).padStart(2, '0')}
                      </td>
                      <td className="py-2.5">
                        <span className="flex items-center gap-2.5">
                          {entry.track.album_image_url ? (
                            <img
                              src={entry.track.album_image_url}
                              alt=""
                              className="size-8 border border-line object-cover"
                            />
                          ) : (
                            <span className="grid size-8 place-items-center border border-line bg-paper font-serif text-sm text-muted/60">
                              ♪
                            </span>
                          )}
                          <span className="min-w-0">
                            <span className="block truncate font-medium leading-tight">
                              {entry.track.title}
                            </span>
                            <span className="block truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                              {entry.track.artists}
                            </span>
                          </span>
                        </span>
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {Math.round(entry.rating)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(entry.comparisons)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(entry.wins)}
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
                {first}–{last} of {formatNumber(data.total)}
              </span>
              <button
                type="button"
                onClick={() => setOffset(offset + PAGE_SIZE)}
                disabled={last >= data.total}
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
