import { useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminVoterDetail } from '../../lib/types'
import { DuelRows } from './AdminDuels'
import {
  AsOf,
  BarList,
  formatDecimal,
  formatNumber,
  formatPercent,
  formatSeconds,
  formatWhen,
  HeatmapGrid,
  StatTile,
} from './AdminStatsUI'

function SessionBandChart({ detail }: { detail: AdminVoterDetail }) {
  const rows = detail.decisions_by_session
  if (rows.length === 0) {
    return <p className="text-sm text-muted">No sessions yet.</p>
  }
  const width = 640
  const height = 150
  const padding = { top: 10, right: 12, bottom: 22, left: 40 }
  const innerWidth = width - padding.left - padding.right
  const innerHeight = height - padding.top - padding.bottom
  const values = rows.flatMap((row) =>
    [row.p10_decision_seconds, row.median_decision_seconds, row.p90_decision_seconds].filter(
      (value): value is number => value != null,
    ),
  )
  const maxY = Math.max(1, ...values)
  const sx = (index: number) =>
    padding.left + (rows.length === 1 ? innerWidth / 2 : (index / (rows.length - 1)) * innerWidth)
  const sy = (value: number) => padding.top + innerHeight - (value / maxY) * innerHeight

  const bandTop = rows
    .map((row, index) => `${sx(index)},${sy(row.p90_decision_seconds ?? maxY)}`)
    .join(' ')
  const bandBottom = [...rows]
    .reverse()
    .map((row, reverseIndex) => {
      const index = rows.length - 1 - reverseIndex
      return `${sx(index)},${sy(row.p10_decision_seconds ?? 0)}`
    })
    .join(' ')
  const medianPath = rows
    .map((row, index) => {
      const value = row.median_decision_seconds ?? 0
      return `${index === 0 ? 'M' : 'L'} ${sx(index).toFixed(1)} ${sy(value).toFixed(1)}`
    })
    .join(' ')

  return (
    <svg viewBox={`0 0 ${width} ${height}`} className="w-full" role="img" aria-label="Decision time by session">
      <line
        x1={padding.left}
        x2={width - padding.right}
        y1={padding.top + innerHeight}
        y2={padding.top + innerHeight}
        stroke="var(--color-line)"
      />
      <polygon points={`${bandTop} ${bandBottom}`} fill="var(--color-cobalt)" opacity="0.12" />
      <path d={medianPath} fill="none" stroke="var(--color-cobalt)" strokeWidth="2" />
      <text x={padding.left - 4} y={padding.top + 6} textAnchor="end" className="fill-muted font-mono text-[8px]">
        {maxY.toFixed(0)}s
      </text>
      <text x={padding.left - 4} y={padding.top + innerHeight} textAnchor="end" className="fill-muted font-mono text-[8px]">
        0s
      </text>
      {rows.map((row, index) => (
        <circle
          key={row.session}
          cx={sx(index)}
          cy={sy(row.median_decision_seconds ?? 0)}
          r="2.5"
          fill="var(--color-cobalt)"
        />
      ))}
      <text x={padding.left} y={height - 5} className="fill-muted font-mono text-[8px]">
        Session 1
      </text>
      <text x={width - padding.right} y={height - 5} textAnchor="end" className="fill-muted font-mono text-[8px]">
        Session {rows.length}
      </text>
    </svg>
  )
}

export function AdminVoterDetail({
  userId,
  onClose,
  timezone,
}: {
  userId: string
  onClose: () => void
  timezone: string
}) {
  const [data, setData] = useState<AdminVoterDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setData(await api.getAdminVoterDetail(userId, timezone, 10))
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load this voter')
    } finally {
      setLoading(false)
    }
  }, [timezone, userId])

  useEffect(() => {
    void load()
  }, [load])

  useEffect(() => {
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  const voter = data?.voter

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-ink/35 p-0 sm:p-4">
      <button
        type="button"
        aria-label="Close voter panel"
        onClick={onClose}
        className="absolute inset-0 cursor-default"
      />
      <section
        role="dialog"
        aria-modal="true"
        aria-label="Voter detail"
        className="relative z-10 h-full w-full max-w-4xl overflow-y-auto border-l-2 border-ink bg-card p-4 shadow-[-6px_0_0_0_var(--color-ink)] sm:rounded-2xl sm:border-2 sm:p-5"
      >
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="pv-admin-label">Voter detail</p>
            <h2 className="font-serif text-2xl leading-tight tracking-[-0.02em]">
              {voter?.username ?? 'Loading…'}
            </h2>
            <p className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
              {voter
                ? `${formatNumber(voter.votes)} votes · ${formatNumber(voter.sessions)} sessions · last active ${formatWhen(voter.last_active_at)}`
                : ' '}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <AsOf value={data?.as_of} />
            <button type="button" onClick={onClose} className="pv-admin-button">
              Close
            </button>
          </div>
        </div>

        {error && <p className="mt-3 text-sm text-danger">{error}</p>}
        {loading && !data ? (
          <div className="mt-4 h-40 animate-pulse rounded-2xl bg-ink/10" />
        ) : data && voter ? (
          <div className="mt-4 space-y-4">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              <StatTile
                label="Median decision"
                value={formatSeconds(voter.median_decision_seconds)}
                hint={`P10 ${formatSeconds(voter.p10_decision_seconds)} · P90 ${formatSeconds(voter.p90_decision_seconds)}`}
              />
              <StatTile
                label="Agreement"
                value={formatPercent(voter.agreement_pct, 1)}
                hint="Picks the currently higher-rated song. Random is about 50%."
              />
              <StatTile
                label="No-vote rate"
                value={formatPercent(voter.no_vote_rate, 1)}
                hint={`${formatNumber(voter.duels)} duels shown · ${formatNumber(voter.votes)} voted`}
              />
              <StatTile
                label="Side bias"
                value={formatPercent(voter.side_bias_pct, 1)}
                hint="Share of picks on the left. Unbiased is about 50%."
              />
            </div>

            <div className="flex flex-wrap gap-2">
              {voter.flags.fast_clicker && (
                <span className="pv-admin-chip border-danger/40 text-danger">
                  Fast clicker
                </span>
              )}
              {voter.flags.side_bias && (
                <span className="pv-admin-chip border-danger/40 text-danger">
                  Side bias
                </span>
              )}
              {voter.flags.low_agreement && (
                <span className="pv-admin-chip border-danger/40 text-danger">
                  Low agreement
                </span>
              )}
              {!voter.flags.fast_clicker &&
                !voter.flags.side_bias &&
                !voter.flags.low_agreement && (
                  <span className="pv-admin-chip border-ok/40 text-ok">
                    No flags
                  </span>
                )}
              <span className="pv-admin-chip">
                {formatNumber(voter.distinct_songs_seen)} distinct songs
              </span>
              <span className="pv-admin-chip">
                total voting time {formatSeconds(voter.total_voting_time_seconds)}
              </span>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="pv-admin-panel">
                <p className="pv-admin-label">Voted duels only</p>
                <h3 className="font-serif text-xl">Decision-time histogram</h3>
                <div className="mt-3">
                  <BarList
                    data={data.decision_histogram.map((bucket) => ({
                      label: bucket.label,
                      value: bucket.count,
                    }))}
                    valueLabel={(value) => `${formatNumber(value)} votes`}
                  />
                </div>
              </div>
              <div className="pv-admin-panel">
                <p className="pv-admin-label">Session trend</p>
                <h3 className="font-serif text-xl">Faster or slower over days?</h3>
                <div className="mt-2">
                  <SessionBandChart detail={data} />
                </div>
                <p className="text-[11px] leading-relaxed text-muted">
                  The line is each session&apos;s median decision time; the band is
                  P10–P90.
                </p>
              </div>
            </div>

            <div className="pv-admin-panel">
              <p className="pv-admin-label">Position inside each session</p>
              <h3 className="font-serif text-xl">Within-session tiredness</h3>
              <div className="mt-3 overflow-x-auto">
                <table className="pv-admin-table min-w-[520px]">
                  <thead>
                    <tr>
                      <th>Duel position</th>
                      <th className="text-right">Decisions</th>
                      <th className="text-right">Median time</th>
                      <th className="text-right">Agreement</th>
                      <th className="text-right">No-vote rate</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.tiredness.map((row) => (
                      <tr key={row.label}>
                        <td className="font-mono text-xs">{row.label}</td>
                        <td className="text-right font-mono text-xs">{formatNumber(row.decisions)}</td>
                        <td className="text-right font-mono text-xs">{formatSeconds(row.median_decision_seconds)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.agreement_pct, 1)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.no_vote_rate, 1)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                If the median time drops and agreement falls later in a session,
                the voter may be rushing or losing focus.
              </p>
            </div>

            <div className="pv-admin-panel">
              <p className="pv-admin-label">Every run of activity</p>
              <h3 className="font-serif text-xl">Sessions</h3>
              <div className="mt-3 overflow-x-auto">
                <table className="pv-admin-table min-w-[820px]">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>Start</th>
                      <th className="text-right">Duration</th>
                      <th className="text-right">Duels</th>
                      <th className="text-right">Votes</th>
                      <th className="text-right">No vote</th>
                      <th className="text-right">Median</th>
                      <th className="text-right">First 10</th>
                      <th className="text-right">Last 10</th>
                      <th className="text-right">Agreement</th>
                      <th className="text-right">Side bias</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.sessions.map((row) => (
                      <tr key={row.session}>
                        <td className="font-serif text-lg text-muted">{String(row.session).padStart(2, '0')}</td>
                        <td className="whitespace-nowrap font-mono text-[9px] text-muted">{formatWhen(row.started_at)}</td>
                        <td className="text-right font-mono text-xs">{formatSeconds(row.duration_seconds)}</td>
                        <td className="text-right font-mono text-xs">{formatNumber(row.duels)}</td>
                        <td className="text-right font-mono text-xs">{formatNumber(row.votes)}</td>
                        <td className="text-right font-mono text-xs">{formatNumber(row.no_votes)}</td>
                        <td className="text-right font-mono text-xs">{formatSeconds(row.median_decision_seconds)}</td>
                        <td className="text-right font-mono text-xs">{formatSeconds(row.first10_decision_seconds)}</td>
                        <td className="text-right font-mono text-xs">{formatSeconds(row.last10_decision_seconds)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.agreement_pct, 1)}</td>
                        <td className="text-right font-mono text-xs">{formatPercent(row.side_bias_pct, 1)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            <div className="pv-admin-panel">
              <p className="pv-admin-label">Duels shown · local time</p>
              <h3 className="font-serif text-xl">Activity heatmap</h3>
              <div className="mt-3">
                <HeatmapGrid heatmap={data.heatmap} label="duels" />
              </div>
            </div>

            <div>
              <div className="mb-2 flex items-end justify-between gap-2">
                <div>
                  <p className="pv-admin-label">Filtered to this voter</p>
                  <h3 className="font-serif text-xl">10 most recent duels</h3>
                </div>
                <span className="pv-admin-chip">{formatDecimal(voter.avg_duels_per_session, 1)} duels / session</span>
              </div>
              <DuelRows duels={data.recent_duels.duels} />
            </div>
          </div>
        ) : null}
      </section>
    </div>
  )
}
