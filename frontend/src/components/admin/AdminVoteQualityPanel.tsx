import { useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminVoteQuality } from '../../lib/types'
import { localTimezone } from '../../lib/time'
import {
  AsOf,
  formatNumber,
  formatPercent,
  formatSeconds,
  HeatmapGrid,
  StatTile,
} from './AdminStatsUI'

export function AdminVoteQualityPanel() {
  const [data, setData] = useState<AdminVoteQuality | null>(null)
  const [error, setError] = useState<string | null>(null)
  const timezone = localTimezone()

  useEffect(() => {
    let active = true
    api
      .getAdminVoteQuality(timezone)
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load vote quality')
        }
      })
    return () => {
      active = false
    }
  }, [timezone])

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Can we trust the vote?</p>
          <h3 className="font-serif text-xl text-ink">Vote quality</h3>
        </div>
        <AsOf value={data?.as_of} />
      </div>

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}

      {data ? (
        <>
          <div className="mt-3 grid gap-2 sm:grid-cols-2 xl:grid-cols-3">
            <StatTile
              label={`Upset rate · below ${formatPercent(data.upset_threshold_pct, 0)}`}
              value={formatPercent(data.upset_rate, 1)}
              hint="Share of votes where the winner was the underdog. A healthy room keeps this low but not zero; 10–30% is normal."
            />
            <StatTile
              label="No-vote rate"
              value={formatPercent(data.no_vote_rate, 1)}
              hint={`${formatNumber(data.total_duels - data.total_votes)} shown duels have no vote. Includes skips, undo and the duel on screen; under ~10% is healthy, and this is an upper bound on skipping.`}
            />
            <StatTile
              label="Median decision time"
              value={formatSeconds(data.median_decision_seconds)}
              hint={`P90 ${formatSeconds(data.p90_decision_seconds)}. ${formatNumber(data.idle_excluded)} idle decisions over the session gap were excluded. A healthy median is roughly 2–10s.`}
            />
            <StatTile
              label="Left vs right wins"
              value={`${formatPercent(data.left_win_pct, 1)} / ${formatPercent(data.right_win_pct, 1)}`}
              hint="Position bias. A healthy value is close to 50 / 50; a large gap means the side itself is influencing picks."
            />
            <StatTile
              label="Flagged voters"
              value={formatNumber(data.flagged_voters)}
              hint={`${formatPercent(data.flagged_vote_share_pct, 1)} of all votes come from voters with at least one informational flag. A healthy room keeps this well below half.`}
              tone={data.flagged_vote_share_pct > 40 ? 'danger' : undefined}
            />
            <StatTile
              label="Votes counted"
              value={formatNumber(data.total_votes)}
              hint={`${formatNumber(data.total_duels)} duels were shown in the active playlist.`}
            />
          </div>

          <div className="mt-4">
            <div className="mb-2 flex items-end justify-between gap-2">
              <div>
                <p className="pv-admin-label">Weekday × hour · local time</p>
                <h4 className="font-serif text-lg">When the room votes</h4>
              </div>
              <span className="pv-admin-chip">{formatNumber(data.total_votes)} votes</span>
            </div>
            <HeatmapGrid heatmap={data.weekday_hour} label="votes" />
            <p className="mt-1 text-[11px] leading-relaxed text-muted">
              Counts votes by the viewer&apos;s local weekday and hour. A healthy
              spread follows the room&apos;s real listening habits; a single hot
              hour can mean a few people clicked through together.
            </p>
          </div>
        </>
      ) : (
        <div className="mt-3 h-28 animate-pulse rounded-xl bg-ink/10" />
      )}
    </div>
  )
}
