import { useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminSongInsights, AdminTrackBrief } from '../../lib/types'
import {
  AsOf,
  BarList,
  formatDecimal,
  formatNumber,
  formatPercent,
  StatTile,
} from './AdminStatsUI'

function BriefList({
  rows,
  valueLabel,
}: {
  rows: AdminTrackBrief[]
  valueLabel: (value: number) => string
}) {
  if (rows.length === 0) {
    return <p className="mt-1 text-xs text-muted">No songs match.</p>
  }
  return (
    <ol className="mt-1 space-y-1">
      {rows.map((row) => (
        <li key={row.id} className="flex items-baseline justify-between gap-2 text-xs">
          <span className="min-w-0 truncate text-ink" title={`${row.title} — ${row.artists}`}>
            {row.title}
          </span>
          <span className="shrink-0 font-mono text-[9px] text-muted">
            {valueLabel(row.value)}
          </span>
        </li>
      ))}
    </ol>
  )
}

export function AdminSongInsightsPanel({
  onOpenSong,
}: {
  onOpenSong?: (id: string) => void
}) {
  const [data, setData] = useState<AdminSongInsights | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    api
      .getAdminSongInsights()
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load song insights')
        }
      })
    return () => {
      active = false
    }
  }, [])

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Where the signal is weakest</p>
          <h3 className="font-serif text-xl text-ink">Song-level insights</h3>
        </div>
        <AsOf value={data?.as_of} />
      </div>

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}

      {data ? (
        <div className="mt-3 space-y-4">
          <div className="grid gap-3 md:grid-cols-2">
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Highest no-vote rate · min 10 shown</p>
              <BriefList
                rows={data.most_no_voted}
                valueLabel={(value) => formatPercent(value, 1)}
              />
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                These are the songs people skip most. Unknown songs and songs
                without a preview cluster here. No-vote includes undo and the
                duel on screen, so treat it as an upper bound.
              </p>
            </div>

            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Preview impact</p>
              <div className="mt-2 overflow-x-auto">
                <table className="pv-admin-table min-w-[360px]">
                  <thead>
                    <tr>
                      <th>Preview</th>
                      <th className="text-right">Songs</th>
                      <th className="text-right">No vote</th>
                      <th className="text-right">Win %</th>
                      <th className="text-right">Avg rank</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.preview_impact.map((row) => (
                      <tr key={String(row.has_preview)}>
                        <td className="text-xs">{row.has_preview ? 'Available' : 'Missing'}</td>
                        <td className="text-right font-mono text-xs">{formatNumber(row.songs)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.no_vote_rate, 1)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.win_pct, 1)}</td>
                        <td className="text-right font-mono text-xs">{formatDecimal(row.average_rank, 1)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                If songs without a preview have a much higher no-vote rate and a
                worse win rate, preview coverage is shaping the standings.
              </p>
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-2">
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Wins minus expected · min 10 votes</p>
              <p className="mt-1 font-serif text-lg">Overperformers</p>
              <BriefList
                rows={data.overperformers.map((row) => ({
                  id: row.id,
                  title: row.title,
                  artists: row.artists,
                  album_image_url: row.album_image_url,
                  value: row.form,
                }))}
                valueLabel={(value) => `+${value.toFixed(1)}`}
              />
            </div>
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Wins minus expected · min 10 votes</p>
              <p className="mt-1 font-serif text-lg">Underperformers</p>
              <BriefList
                rows={data.underperformers.map((row) => ({
                  id: row.id,
                  title: row.title,
                  artists: row.artists,
                  album_image_url: row.album_image_url,
                  value: row.form,
                }))}
                valueLabel={(value) => value.toFixed(1)}
              />
            </div>
          </div>

          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            <div className="rounded-xl border border-line bg-paper p-3">
              <StatTile
                label="Never voted"
                value={formatNumber(data.never_voted_count)}
                hint="Songs with zero voted duels. They show at 1000 until they get a vote."
              />
              <p className="mt-2 pv-admin-label text-[8px]">First 20 by title</p>
              <BriefList
                rows={data.never_voted}
                valueLabel={() => '0 duels'}
              />
            </div>
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Fewest duels</p>
              <BriefList
                rows={data.fewest_duels}
                valueLabel={(value) => `${formatNumber(value)} duels`}
              />
            </div>
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Rating distribution</p>
              <div className="mt-2">
                <BarList
                  data={data.rating_histogram.map((bucket) => ({
                    label: bucket.label,
                    value: bucket.count,
                  }))}
                  valueLabel={(value) => `${formatNumber(value)} songs`}
                />
              </div>
            </div>
            <div className="rounded-xl border border-line bg-paper p-3">
              <p className="pv-admin-label">Duels per song</p>
              <div className="mt-2">
                <BarList
                  data={data.duels_histogram.map((bucket) => ({
                    label: bucket.label,
                    value: bucket.count,
                  }))}
                  valueLabel={(value) => `${formatNumber(value)} songs`}
                />
              </div>
            </div>
          </div>

          <p className="text-[11px] leading-relaxed text-muted">
            Click a song name in the standings or the duel feed to open its full
            deep dive, including its rating line and complete duel history.
          </p>
          <button
            type="button"
            onClick={() => {
              const first = data.overperformers[0] ?? data.most_no_voted[0]
              if (first) onOpenSong?.(first.id)
            }}
            disabled={data.overperformers.length === 0 && data.most_no_voted.length === 0}
            className="pv-admin-button"
          >
            Open an example song
          </button>
        </div>
      ) : (
        <div className="mt-3 h-28 animate-pulse rounded-xl bg-ink/10" />
      )}
    </div>
  )
}
