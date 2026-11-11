import { useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminCrowd as AdminCrowdData } from '../../lib/types'
import {
  AsOf,
  BarList,
  formatNumber,
  formatPercent,
  formatSeconds,
} from './AdminStatsUI'

export function AdminCrowd() {
  const [data, setData] = useState<AdminCrowdData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)
    api
      .getAdminCrowd()
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load crowd stats')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
    }
  }, [])

  if (loading) {
    return (
      <div className="pv-admin-panel">
        <div className="h-5 w-40 animate-pulse rounded-full bg-ink/10" />
        <div className="mt-4 h-28 animate-pulse rounded-xl bg-ink/10" />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Across every voter</p>
          <h2 className="font-serif text-2xl tracking-[-0.02em]">Crowd level</h2>
        </div>
        <AsOf value={data?.as_of} />
      </div>

      {error && <p className="text-sm text-danger">{error}</p>}

      {data && (
        <>
          <div className="grid gap-3 md:grid-cols-3">
            <div className="pv-admin-panel">
              <p className="pv-admin-label">Voter medians</p>
              <h3 className="font-serif text-xl">Decision time</h3>
              <div className="mt-3">
                <BarList
                  data={data.decision_time_distribution.map((bucket) => ({
                    label: bucket.label,
                    value: bucket.count,
                  }))}
                  valueLabel={(value) => `${formatNumber(value)} voters`}
                />
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                A healthy crowd clusters in the 2–10 second range. A large
                sub-second group suggests clicking through without listening.
              </p>
            </div>
            <div className="pv-admin-panel">
              <p className="pv-admin-label">Picks vs current standings</p>
              <h3 className="font-serif text-xl">Agreement</h3>
              <div className="mt-3">
                <BarList
                  data={data.agreement_distribution.map((bucket) => ({
                    label: bucket.label,
                    value: bucket.count,
                  }))}
                  valueLabel={(value) => `${formatNumber(value)} voters`}
                />
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                Random clicking is about 50%. A healthy room mostly sits above
                55–60%, with a few contrarians.
              </p>
            </div>
            <div className="pv-admin-panel">
              <p className="pv-admin-label">Duels shown per run</p>
              <h3 className="font-serif text-xl">Session length</h3>
              <div className="mt-3">
                <BarList
                  data={data.session_length_distribution.map((bucket) => ({
                    label: bucket.label,
                    value: bucket.count,
                  }))}
                  valueLabel={(value) => `${formatNumber(value)} sessions`}
                />
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                Very long sessions are where tiredness shows up. Short sessions
                are normal on phones.
              </p>
            </div>
          </div>

          <div className="pv-admin-panel">
            <p className="pv-admin-label">All voters, all sessions</p>
            <h3 className="font-serif text-xl">Crowd-wide tiredness curve</h3>
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
              If agreement drops and no-votes rise after duel 50, later decisions
              in long sessions carry less signal. The vote is still valid; this is
              context for reading the standings.
            </p>
          </div>
        </>
      )}
    </div>
  )
}
