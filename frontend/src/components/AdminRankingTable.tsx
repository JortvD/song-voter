import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type { AdminStandingsPage } from '../lib/types'
import {
  AdminPagination,
  AsOf,
  formatNumber,
  formatPercent,
  formatWhen,
} from './admin/AdminStatsUI'

const PAGE_SIZE = 50

function RankChange({ value }: { value: number | null }) {
  if (value == null || value === 0) {
    return <span className="text-muted">·</span>
  }
  return (
    <span className={value > 0 ? 'text-ok' : 'text-danger'}>
      {value > 0 ? `▲${value}` : `▼${Math.abs(value)}`}
    </span>
  )
}

function FormValue({ value }: { value: number }) {
  const tone = value > 0.5 ? 'text-ok' : value < -0.5 ? 'text-danger' : 'text-muted'
  return (
    <span className={tone}>
      {value >= 0 ? '+' : ''}
      {value.toFixed(1)}
    </span>
  )
}

export function AdminRankingTable({
  onOpenSong,
}: {
  onOpenSong?: (id: string) => void
}) {
  const [data, setData] = useState<AdminStandingsPage | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [moreColumns, setMoreColumns] = useState(false)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getAdminStandings(offset, PAGE_SIZE, query)
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
          <AsOf value={data?.as_of} />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <button
            type="button"
            onClick={() => setMoreColumns((value) => !value)}
            aria-pressed={moreColumns}
            className={`btn-ghost ${moreColumns ? 'bg-ink text-paper' : ''}`}
          >
            {moreColumns ? 'Fewer columns' : 'More columns'}
          </button>
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
              <table
                className={`w-full text-left text-sm ${moreColumns ? 'min-w-[980px]' : 'min-w-[620px]'}`}
              >
                <thead>
                  <tr className="border-b border-line font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    <th className="w-10 pb-2 font-medium">#</th>
                    <th className="pb-2 font-medium">Song</th>
                    <th className="pb-2 text-right font-medium">Score</th>
                    <th className="pb-2 text-right font-medium">Duels</th>
                    <th className="pb-2 text-right font-medium">Wins</th>
                    {moreColumns && (
                      <>
                        <th className="pb-2 text-right font-medium">24h</th>
                        <th className="pb-2 text-right font-medium">Win %</th>
                        <th className="pb-2 text-right font-medium">Form</th>
                        <th className="pb-2 text-right font-medium">No vote</th>
                        <th className="pb-2 text-right font-medium">Last duel</th>
                      </>
                    )}
                  </tr>
                </thead>
                <tbody>
                  {data.tracks.map((track) => (
                    <tr key={track.id} className="border-b border-line last:border-b-0">
                      <td className="py-2.5 font-serif text-lg text-muted">
                        {String(track.rank).padStart(2, '0')}
                      </td>
                      <td className="py-2.5">
                        <span className="flex items-center gap-2.5">
                          {track.album_image_url ? (
                            <img
                              src={track.album_image_url}
                              alt=""
                              className="size-8 border border-line object-cover"
                            />
                          ) : (
                            <span className="grid size-8 place-items-center border border-line bg-paper font-serif text-sm text-muted/60">
                              ♪
                            </span>
                          )}
                          <span className="min-w-0">
                            <button
                              type="button"
                              onClick={() => onOpenSong?.(track.id)}
                              className="block max-w-full truncate text-left font-medium leading-tight underline decoration-transparent underline-offset-2 hover:decoration-line"
                              title="Open song deep dive"
                            >
                              {track.title}
                            </button>
                            <span className="block truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                              {track.artists}
                              {!track.has_preview ? ' · no preview' : ''}
                            </span>
                          </span>
                        </span>
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {Math.round(track.rating)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(track.comparisons)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(track.wins)}
                      </td>
                      {moreColumns && (
                        <>
                          <td className="py-2.5 text-right font-mono text-[10px]">
                            <RankChange value={track.rank_change_24h} />
                          </td>
                          <td className="py-2.5 text-right font-mono text-[10px]">
                            {formatPercent(track.win_pct, 1)}
                          </td>
                          <td className="py-2.5 text-right font-mono text-[10px]">
                            <FormValue value={track.form} />
                          </td>
                          <td className="py-2.5 text-right font-mono text-[10px]">
                            {formatPercent(track.no_vote_rate, 1)}
                          </td>
                          <td className="whitespace-nowrap py-2.5 text-right font-mono text-[9px] text-muted">
                            {formatWhen(track.last_duel_at)}
                          </td>
                        </>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <AdminPagination
              offset={offset}
              limit={PAGE_SIZE}
              total={data.total}
              onPageChange={setOffset}
            />
            <p className="mt-2 text-[11px] leading-relaxed text-muted">
              Form is wins minus expected wins: positive means a song is
              outperforming its rating, negative means it is underperforming.
              Rank change compares with 24 hours ago.
            </p>
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
