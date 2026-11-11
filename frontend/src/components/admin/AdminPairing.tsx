import { useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminPairingHealth } from '../../lib/types'
import {
  AsOf,
  formatDecimal,
  formatHours,
  formatNumber,
  formatPercent,
  StatTile,
} from './AdminStatsUI'

const WINDOWS = [
  { hours: 1, label: 'Last hour' },
  { hours: 6, label: 'Last 6h' },
  { hours: 24, label: 'Last 24h' },
]

const STATUS_LABEL: Record<string, string> = {
  not_started: 'Not started',
  catching_up_early: 'Catching up (early)',
  catching_up_late: 'Catching up (late)',
  settled: 'Settled',
}

function BucketBars({ bucket }: { bucket: AdminPairingHealth['buckets'][number] }) {
  const rows = [
    { label: 'Served', value: bucket.served_pct, color: 'var(--color-cobalt)' },
    { label: 'Target', value: bucket.target_pct, color: 'var(--color-butter)' },
    { label: 'Unweighted', value: bucket.unweighted_pct, color: 'var(--color-line)' },
  ]
  return (
    <div className="space-y-1">
      {rows.map((row) => (
        <div key={row.label} className="flex items-center gap-2">
          <span className="w-20 shrink-0 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
            {row.label}
          </span>
          <div className="h-2 min-w-0 flex-1 overflow-hidden rounded-full bg-line/40">
            <div
              className="h-full rounded-full"
              style={{
                width: `${Math.min(100, row.value)}%`,
                background: row.color,
              }}
            />
          </div>
          <span className="w-12 shrink-0 text-right font-mono text-[9px] text-ink">
            {row.value.toFixed(1)}%
          </span>
        </div>
      ))}
    </div>
  )
}

export function AdminPairing() {
  const [data, setData] = useState<AdminPairingHealth | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [windowHours, setWindowHours] = useState(24)
  const [since, setSince] = useState('')
  const [lookbackHours, setLookbackHours] = useState(1)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setData(
        await api.getAdminPairing(
          windowHours,
          since ? new Date(since).toISOString() : undefined,
          lookbackHours,
        ),
      )
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load pairing health')
    } finally {
      setLoading(false)
    }
  }, [lookbackHours, since, windowHours])

  useEffect(() => {
    void load()
  }, [load])

  if (loading && !data) {
    return (
      <div className="space-y-3">
        <div className="h-24 animate-pulse rounded-2xl bg-ink/10" />
        <div className="h-48 animate-pulse rounded-2xl bg-ink/10" />
      </div>
    )
  }

  const settling = data?.settling

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Does the rank weighting work?</p>
          <h2 className="font-serif text-3xl leading-none tracking-[-0.03em]">
            Pairing health
          </h2>
        </div>
        <AsOf value={data?.as_of} />
      </div>

      <div className="flex flex-wrap items-end gap-2 rounded-2xl border-2 border-line bg-card px-3 py-2">
        <label className="block">
          <span className="pv-admin-label text-[8px]">Window</span>
          <select
            value={windowHours}
            onChange={(event) => {
              setWindowHours(Number(event.target.value))
              setSince('')
            }}
            disabled={Boolean(since)}
            className="pv-admin-input mt-1 py-1.5 text-xs"
          >
            {WINDOWS.map((window) => (
              <option key={window.hours} value={window.hours}>
                {window.label}
              </option>
            ))}
          </select>
        </label>
        <label className="block">
          <span className="pv-admin-label text-[8px]">Since (overrides)</span>
          <input
            type="datetime-local"
            value={since}
            onChange={(event) => setSince(event.target.value)}
            className="pv-admin-input mt-1 py-1.5 text-xs"
          />
        </label>
        <label className="block">
          <span className="pv-admin-label text-[8px]">Evenness lookback (h)</span>
          <input
            type="number"
            min={0.1}
            step={0.5}
            value={lookbackHours}
            onChange={(event) => setLookbackHours(Number(event.target.value) || 1)}
            className="pv-admin-input mt-1 w-24 py-1.5 text-xs"
          />
        </label>
        <button type="button" onClick={() => void load()} className="pv-admin-button">
          Refresh
        </button>
      </div>

      {error && <p className="text-sm text-danger">{error}</p>}

      {data && (
        <>
          <div className="grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
            <StatTile
              label="Rank weighting"
              value={data.enabled ? 'On' : 'Off'}
              hint={`strength ${formatDecimal(data.strength, 1)} · scale ${formatNumber(data.scale)} · min avg ${formatDecimal(data.min_avg, 1)}`}
              tone={data.enabled ? 'ok' : 'danger'}
            />
            <StatTile
              label="Average comparisons"
              value={formatDecimal(data.average_comparisons, 1)}
              hint={`minimum average to start weighting: ${formatDecimal(data.min_avg, 1)}`}
              tone={data.average_comparisons >= data.min_avg ? 'ok' : undefined}
            />
            <StatTile
              label="Duels in window"
              value={formatNumber(data.duels_in_window)}
              hint={`${formatNumber(data.window_hours)}h window${data.duels_in_window < 1000 ? ' · low sample, forecast is noisy' : ''}`}
              tone={data.duels_in_window < 1000 ? 'danger' : undefined}
            />
            <StatTile
              label="Bottom share"
              value={formatPercent(settling?.bottom_share_pct ?? 0, 1)}
              hint={`target ${formatPercent(settling?.bottom_target_pct ?? 0, 1)} for #1001+`}
            />
          </div>

          <div className="pv-admin-panel">
            <div className="flex flex-wrap items-end justify-between gap-2">
              <div>
                <p className="pv-admin-label">Rank-weighted aftermath</p>
                <h3 className="font-serif text-xl">Rank buckets</h3>
              </div>
              <p className="max-w-xl text-[11px] leading-relaxed text-muted">
                Served is the bucket&apos;s share of duel appearances in the
                window. Target is its share of the rank weight. Unweighted is its
                share of songs. The algorithm tries to equalise comparisons ÷
                weight.
              </p>
            </div>

            <div className={`mt-3 rounded-xl border px-3 py-2 text-xs ${
              data.verdict_key === 'settled'
                ? 'border-ok/40 bg-ok/5 text-ok'
                : data.verdict_key === 'not_active'
                  ? 'border-line bg-paper text-muted'
                  : 'border-butter bg-butter/10 text-ink'
            }`}>
              {data.verdict}
            </div>

            <div className="mt-4 space-y-3">
              {data.buckets.map((bucket) => (
                <div key={bucket.key} className="rounded-xl border border-line bg-paper p-3">
                  <div className="flex flex-wrap items-baseline justify-between gap-2">
                    <span className="font-serif text-lg">{bucket.label}</span>
                    <span className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                      {formatNumber(bucket.songs)} songs · {formatNumber(bucket.served)} appearances
                    </span>
                  </div>
                  <div className="mt-2 grid gap-3 md:grid-cols-[1fr_auto] md:items-center">
                    <BucketBars bucket={bucket} />
                    <div className="grid grid-cols-2 gap-x-4 gap-y-1 font-mono text-[9px] uppercase tracking-[0.08em] text-muted">
                      <span>avg duels / song</span>
                      <span className="text-right text-ink">{formatDecimal(bucket.avg_duels_per_song, 1)}</span>
                      <span>duels / weight</span>
                      <span className="text-right text-ink">{formatDecimal(bucket.duels_per_weight, 2)}</span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="grid gap-4 lg:grid-cols-2">
            <div className="pv-admin-panel">
              <p className="pv-admin-label">P10 ÷ P90 of comparisons ÷ weight</p>
              <h3 className="font-serif text-xl">Settling</h3>
              <div className="mt-3 grid grid-cols-2 gap-2">
                <StatTile
                  label="Evenness now"
                  value={settling?.even_now == null ? '—' : settling.even_now.toFixed(3)}
                  hint={`${formatHours(settling?.lookback_hours ?? 0)} lookback: ${settling?.even_then == null ? '—' : settling.even_then.toFixed(3)}`}
                />
                <StatTile
                  label="Status"
                  value={settling ? STATUS_LABEL[settling.status] ?? settling.status : '—'}
                  hint={`below 0.5 early, 0.8 late, above settled`}
                  tone={settling?.status === 'settled' ? 'ok' : undefined}
                />
                <StatTile
                  label="Estimated duels left"
                  value={settling?.duels_left == null ? '—' : formatNumber(settling.duels_left)}
                  hint={`at ${formatDecimal(settling?.pace_per_hour ?? 0, 1)} duels/h`}
                />
                <StatTile
                  label="Estimated time left"
                  value={formatHours(settling?.hours_left)}
                  hint={settling?.warn_low_sample ? 'Window has fewer than 1,000 duels: treat as a rough hint.' : 'At the window pace.'}
                  tone={settling?.warn_low_sample ? 'danger' : undefined}
                />
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                Evenness is P10 ÷ P90 of comparisons ÷ weight across all songs.
                It rises toward 1 as the algorithm equalises the field. The
                forecast only appears while evenness is still improving and below
                0.8; it is a straight-line extrapolation, not a promise.
              </p>
            </div>

            <div className="pv-admin-panel">
              <p className="pv-admin-label">Served duels in the window</p>
              <h3 className="font-serif text-xl">Pairing quality</h3>
              <div className="mt-3 grid grid-cols-2 gap-2">
                <StatTile
                  label="Median rating gap"
                  value={formatDecimal(data.quality.median_rating_gap, 1)}
                  hint="Current-rating difference between the two songs."
                />
                <StatTile
                  label="P90 rating gap"
                  value={formatDecimal(data.quality.p90_rating_gap, 1)}
                  hint="Nine in ten duels are closer than this."
                />
                <StatTile
                  label="Same rank bucket"
                  value={formatPercent(data.quality.same_bucket_pct, 1)}
                  hint="Share of duels where both songs are in the same bucket."
                />
                <StatTile
                  label="Duels measured"
                  value={formatNumber(data.quality.duels)}
                  hint={`Window ${formatNumber(data.window_hours)}h`}
                />
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-muted">
                Smaller rating gaps mean closer, more informative duels. A high
                same-bucket share means the engine is pairing like with like.
              </p>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
