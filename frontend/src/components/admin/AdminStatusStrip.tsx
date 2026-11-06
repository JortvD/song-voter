import { Link } from 'react-router-dom'

import type { AdminInsights } from '../../lib/types'

type Props = {
  insights: AdminInsights | null
  loading: boolean
  error: string | null
  onRefresh: () => void
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-20">
      <p className="pv-admin-label text-[8px]">{label}</p>
      <p className="mt-1 font-serif text-xl leading-none text-ink">{value}</p>
    </div>
  )
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminStatusStrip({ insights, loading, error, onRefresh }: Props) {
  const statusLabel = error
    ? 'Needs attention'
    : insights
      ? 'All systems go'
      : 'Waiting for data'
  const statusColor = error ? 'bg-danger' : insights ? 'bg-ok' : 'bg-butter'

  return (
    <section className="pv-admin-panel">
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="flex min-w-0 items-center gap-3">
          <span className="grid size-14 shrink-0 place-items-center overflow-hidden rounded-xl border border-line bg-paper">
            {insights?.playlist_image_url ? (
              <img
                src={insights.playlist_image_url}
                alt=""
                className="size-full object-cover"
              />
            ) : (
              <span className="font-serif text-2xl text-muted">♪</span>
            )}
          </span>
          <div className="min-w-0">
            <p className="pv-admin-label">Mission control</p>
            <h1 className="truncate font-serif text-3xl leading-none tracking-[-0.03em] text-ink">
              {insights?.playlist_name ?? 'No tape loaded'}
            </h1>
            <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {insights?.revealed ? 'Standings live' : 'Standings sealed'} ·{' '}
              {insights?.total_voters ?? 0} voters · {insights?.total_picks ?? 0} picks
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <div className="hidden items-center gap-5 border-l border-line pl-5 lg:flex">
            <MiniStat
              label="Picks today"
              value={formatNumber(insights?.picks_today ?? 0)}
            />
            <MiniStat
              label="Active 24h"
              value={formatNumber(insights?.active_voters_24h ?? 0)}
            />
            <MiniStat
              label="Coverage"
              value={`${Math.round(insights?.coverage_pct ?? 0)}%`}
            />
            <MiniStat
              label="Duels"
              value={formatNumber(insights?.total_duels ?? 0)}
            />
          </div>

          <span className="pv-admin-chip">
            <span className={`size-1.5 rounded-full ${statusColor}`} />
            {statusLabel}
          </span>

          <button
            type="button"
            onClick={onRefresh}
            className="pv-admin-button"
          >
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
          <Link to="/standings" className="pv-admin-button">
            Public standings
          </Link>
        </div>
      </div>
    </section>
  )
}
