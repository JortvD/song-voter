import { useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminProgressForecast, AdminRankMove } from '../../lib/types'
import {
  AsOf,
  formatDecimal,
  formatHours,
  formatNumber,
  StatTile,
  TrendLine,
} from './AdminStatsUI'

function MoveList({ rows, title }: { rows: AdminRankMove[]; title: string }) {
  return (
    <div>
      <p className="pv-admin-label text-[8px]">{title}</p>
      {rows.length === 0 ? (
        <p className="mt-1 text-xs text-muted">No movement.</p>
      ) : (
        <ol className="mt-1 space-y-1">
          {rows.map((row) => (
            <li key={row.id} className="flex items-baseline justify-between gap-2 text-xs">
              <span className="min-w-0 truncate text-ink" title={`${row.title} — ${row.artists}`}>
                {row.title}
              </span>
              <span
                className={`shrink-0 font-mono text-[9px] ${row.change > 0 ? 'text-ok' : 'text-danger'}`}
              >
                {row.change > 0 ? `▲${row.change}` : `▼${Math.abs(row.change)}`}
              </span>
            </li>
          ))}
        </ol>
      )}
    </div>
  )
}

export function AdminProgressPanel() {
  const [data, setData] = useState<AdminProgressForecast | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    api
      .getAdminProgress()
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load progress')
        }
      })
    return () => {
      active = false
    }
  }, [])

  const ranking = data?.ranking
  const progress = Math.min(100, Math.max(0, data?.progress_pct ?? 0))

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Will the vote settle in time?</p>
          <h3 className="font-serif text-xl text-ink">Progress &amp; forecast</h3>
        </div>
        <AsOf value={data?.as_of} />
      </div>

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}

      {data ? (
        <>
          <div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
            <StatTile
              label="Pace · 24h"
              value={`${formatDecimal(data.duels_per_hour_24h, 1)}/h`}
              hint={`${formatDecimal(data.duels_per_hour_7d, 1)}/h over the last 7 days.`}
            />
            <StatTile
              label="Duels per song"
              value={`${formatDecimal(data.avg_duels_per_song, 1)}`}
              hint={`Minimum ${formatNumber(data.min_duels_per_song)} · target ${formatDecimal(data.target_duels_per_song, 1)}.`}
            />
            <StatTile
              label="Songs at target"
              value={`${formatNumber(data.songs_at_or_above_target)} / ${formatNumber(data.total_songs)}`}
              hint="Songs with at least the target number of voted duels."
            />
            {data.scheduled_reveal_at ? (
              <StatTile
                label="At reveal"
                value={
                  data.projected_avg_duels_per_song == null
                    ? '—'
                    : `${formatDecimal(data.projected_avg_duels_per_song, 1)} / song`
                }
                hint={
                  data.target_reached_at_reveal
                    ? 'The target should be reached at the current 7-day pace.'
                    : 'The target is not projected to be reached at the current 7-day pace.'
                }
                tone={data.target_reached_at_reveal ? 'ok' : 'danger'}
              />
            ) : (
              <StatTile
                label="ETA to target"
                value={formatHours(data.eta_hours)}
                hint="At the current 7-day voting pace, ignoring pauses and reveal scheduling."
              />
            )}
          </div>

          <div className="mt-3">
            <div className="flex items-center justify-between gap-2 font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
              <span>Average duels per song</span>
              <span className="text-ink">
                {formatDecimal(data.avg_duels_per_song, 1)} / {formatDecimal(data.target_duels_per_song, 1)}
              </span>
            </div>
            <div className="pv-admin-meter mt-1.5 h-3">
              <div
                className="pv-admin-meter-fill"
                style={{
                  width: `${progress}%`,
                  background:
                    progress >= 100 ? 'var(--color-ok)' : 'var(--color-cobalt)',
                }}
              />
            </div>
            <p className="mt-1 text-[11px] leading-relaxed text-muted">
              {data.scheduled_reveal_at
                ? `Reveal scheduled for ${new Date(data.scheduled_reveal_at).toLocaleString()}. Projected total: ${formatNumber(data.projected_total_duels)} duels.`
                : 'No reveal is scheduled, so the ETA assumes voting continues at the 7-day pace.'}
            </p>
          </div>

          {ranking && (
            <div className="mt-4 border-t border-line pt-3">
              <div className="flex flex-wrap items-end justify-between gap-2">
                <div>
                  <p className="pv-admin-label">Convergence</p>
                  <h4 className="font-serif text-lg">Ranking stability</h4>
                </div>
                <span className="pv-admin-chip">
                  avg move {formatDecimal(ranking.average_abs_change_24h, 1)} places / 24h
                </span>
              </div>
              <div className="mt-2 grid gap-2 sm:grid-cols-3">
                <StatTile
                  label="Top 100 kept · 24h"
                  value={`${formatNumber(ranking.top100_overlap_24h)} / 100`}
                  hint="How many of today's top 100 were also top 100 a day ago. Higher is more stable."
                />
                <StatTile
                  label="Top 100 kept · 7d"
                  value={`${formatNumber(ranking.top100_overlap_7d)} / 100`}
                  hint="Compared with a week ago. This should rise as the vote converges."
                />
                <StatTile
                  label="Avg absolute change"
                  value={formatDecimal(ranking.average_abs_change_24h, 1)}
                  hint="Average number of places the current top 100 moved in 24h. It should shrink over time."
                />
              </div>

              <div className="mt-3 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
                <MoveList rows={ranking.biggest_risers_24h} title="Risers · 24h" />
                <MoveList rows={ranking.biggest_fallers_24h} title="Fallers · 24h" />
                <MoveList rows={ranking.biggest_risers_7d} title="Risers · 7d" />
                <MoveList rows={ranking.biggest_fallers_7d} title="Fallers · 7d" />
              </div>

              <div className="mt-3">
                <p className="pv-admin-label text-[8px]">
                  Average absolute rank change of the top 100 · last 14 days
                </p>
                <TrendLine
                  points={ranking.trend.map((point, index) => ({
                    x: index,
                    y: point.average_abs_change,
                    label: point.date.slice(5),
                  }))}
                  height={110}
                  formatValue={(value) => value.toFixed(1)}
                />
                <p className="text-[11px] leading-relaxed text-muted">
                  A falling line means the standings are settling. A flat or rising
                  line means new votes are still reshuffling the top 100.
                </p>
              </div>
            </div>
          )}
        </>
      ) : (
        <div className="mt-3 h-28 animate-pulse rounded-xl bg-ink/10" />
      )}
    </div>
  )
}
