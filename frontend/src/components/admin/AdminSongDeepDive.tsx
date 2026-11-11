import { useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminSongDetail } from '../../lib/types'
import { DuelRows } from './AdminDuels'
import {
  AdminPagination,
  AsOf,
  formatDecimal,
  formatNumber,
  formatPercent,
  formatSeconds,
  StatTile,
  TrendLine,
} from './AdminStatsUI'

function RankArrow({
  now,
  then,
  label,
}: {
  now: number
  then: number | null
  label: string
}) {
  if (then == null) {
    return (
      <span className="font-mono text-[10px] text-muted">
        {label} — <span className="text-muted">no data</span>
      </span>
    )
  }
  const change = then - now
  const icon = change > 0 ? '▲' : change < 0 ? '▼' : '·'
  const tone = change > 0 ? 'text-ok' : change < 0 ? 'text-danger' : 'text-muted'
  return (
    <span className={`font-mono text-[10px] ${tone}`}>
      {label} #{then} {icon} {change !== 0 ? Math.abs(change) : ''}
    </span>
  )
}

export function SongDeepDive({
  trackId,
  onClose,
  onRemoved,
}: {
  trackId: string
  onClose: () => void
  onRemoved: (message: string) => void
}) {
  const [data, setData] = useState<AdminSongDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [offset, setOffset] = useState(0)
  const [removeOpen, setRemoveOpen] = useState(false)
  const [removeConfirm, setRemoveConfirm] = useState('')
  const [removing, setRemoving] = useState(false)
  const [removeError, setRemoveError] = useState<string | null>(null)
  const pageSize = 15

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setData(await api.getAdminSong(trackId, offset, pageSize))
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load song detail')
    } finally {
      setLoading(false)
    }
  }, [offset, trackId])

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

  async function removeTrack() {
    if (!data || removing) return
    if (removeConfirm.trim() !== 'REMOVE') {
      setRemoveError('Type REMOVE to confirm.')
      return
    }

    setRemoving(true)
    setRemoveError(null)
    try {
      const result = await api.removeAdminTrack(trackId, removeConfirm)
      onRemoved(
        `Removed “${result.title}” by ${result.artists}. Deleted ${result.deleted_votes.toLocaleString()} votes and ${result.deleted_duels.toLocaleString()} duels, then rebuilt ${result.tracks_updated.toLocaleString()} remaining tracks.`,
      )
    } catch (caught) {
      setRemoveError(
        caught instanceof ApiError ? caught.message : 'Could not remove this track',
      )
    } finally {
      setRemoving(false)
    }
  }

  const record = data?.record
  const formZone =
    record == null ? '' : record.form > 1 ? 'outperforming its rating' : record.form < -1 ? 'underperforming its rating' : 'matching its rating'

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-ink/35 p-0 sm:p-4">
      <button
        type="button"
        aria-label="Close song panel"
        onClick={onClose}
        className="absolute inset-0 cursor-default"
      />
      <section
        role="dialog"
        aria-modal="true"
        aria-label="Song deep dive"
        className="relative z-10 h-full w-full max-w-4xl overflow-y-auto border-l-2 border-ink bg-card p-4 shadow-[-6px_0_0_0_var(--color-ink)] sm:rounded-2xl sm:border-2 sm:p-5"
      >
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            {data?.track.album_image_url ? (
              <img
                src={data.track.album_image_url}
                alt=""
                className="size-16 shrink-0 rounded-xl border border-line object-cover"
              />
            ) : (
              <span className="grid size-16 shrink-0 place-items-center rounded-xl border border-line bg-paper font-serif text-xl text-muted">
                ♪
              </span>
            )}
            <div className="min-w-0">
              <p className="pv-admin-label">Song deep dive</p>
              <h2 className="truncate font-serif text-2xl leading-tight tracking-[-0.02em] text-ink">
                {data?.track.title ?? 'Loading…'}
              </h2>
              <p className="truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                {data?.track.artists}
                {data?.track.release_year ? ` · ${data.track.release_year}` : ''}
                {data?.track.album ? ` · ${data.track.album}` : ''}
              </p>
            </div>
          </div>
          <div className="flex shrink-0 flex-wrap items-center gap-2">
            <AsOf value={data?.as_of} />
            <button
              type="button"
              onClick={() => {
                setRemoveError(null)
                setRemoveConfirm('')
                setRemoveOpen(true)
              }}
              disabled={!data}
              className="pv-admin-button pv-admin-button-danger"
            >
              Remove track
            </button>
            <button type="button" onClick={onClose} className="pv-admin-button">
              Close
            </button>
          </div>
        </div>

        {error && <p className="mt-3 text-sm text-danger">{error}</p>}
        {loading && !data ? (
          <div className="mt-4 h-40 animate-pulse rounded-2xl bg-ink/10" />
        ) : data && record ? (
          <div className="mt-4 space-y-4">
            <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
              <StatTile
                label="Current rank"
                value={`#${data.track.current_rank}`}
                hint={`of ${formatNumber(data.track.total_tracks)} · rating ${formatDecimal(data.track.current_rating)}`}
              />
              <StatTile
                label="Record"
                value={`${formatNumber(record.wins)}–${formatNumber(record.losses)}`}
                hint={`${formatPercent(record.win_pct, 1)} win rate`}
              />
              <StatTile
                label="Expected wins"
                value={formatDecimal(record.expected_wins)}
                hint={`Actual ${formatNumber(record.wins)} · ${formZone}`}
                tone={record.form > 1 ? 'ok' : record.form < -1 ? 'danger' : undefined}
              />
              <StatTile
                label="No vote"
                value={formatPercent(record.no_vote_rate, 1)}
                hint={`${formatNumber(record.no_votes)} of ${formatNumber(record.shown_duels)} shown`}
              />
            </div>

            <div className="flex flex-wrap items-center gap-3 rounded-xl border border-line bg-paper px-3 py-2">
              <RankArrow now={data.track.current_rank} then={data.rank_24h} label="24h" />
              <RankArrow now={data.track.current_rank} then={data.rank_7d} label="7d" />
              <span className="font-mono text-[10px] text-muted">
                showdowns {formatNumber(record.duels)} · playlist avg {formatDecimal(record.playlist_avg_duels, 1)}
              </span>
              <span className="font-mono text-[10px] text-muted">
                median decision {formatSeconds(record.median_decision_seconds)}
              </span>
              <span className="font-mono text-[10px] text-muted">
                upsets won {formatNumber(record.upset_wins)} · lost {formatNumber(record.upset_losses)}
              </span>
              <span className="font-mono text-[10px] uppercase tracking-[0.1em] text-muted">
                {record.has_preview ? 'preview available' : 'no preview'}
              </span>
            </div>

            <div className="grid gap-3 md:grid-cols-2">
              <div className="rounded-xl border border-line bg-paper p-3">
                <p className="pv-admin-label">Best win</p>
                {data.best_win ? (
                  <p className="mt-1 text-sm text-ink">
                    <span className="font-medium">{data.best_win.title}</span>
                    <span className="block font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                      #{data.best_win.rank} · rating {formatDecimal(data.best_win.rating)} · {new Date(data.best_win.at).toLocaleDateString()}
                    </span>
                  </p>
                ) : (
                  <p className="mt-1 text-sm text-muted">No win yet.</p>
                )}
              </div>
              <div className="rounded-xl border border-line bg-paper p-3">
                <p className="pv-admin-label">Worst loss</p>
                {data.worst_loss ? (
                  <p className="mt-1 text-sm text-ink">
                    <span className="font-medium">{data.worst_loss.title}</span>
                    <span className="block font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                      #{data.worst_loss.rank} · rating {formatDecimal(data.worst_loss.rating)} · {new Date(data.worst_loss.at).toLocaleDateString()}
                    </span>
                  </p>
                ) : (
                  <p className="mt-1 text-sm text-muted">No loss yet.</p>
                )}
              </div>
            </div>

            <div className="pv-admin-panel">
              <div className="flex items-end justify-between gap-2">
                <div>
                  <p className="pv-admin-label">Voted duels only</p>
                  <h3 className="font-serif text-xl text-ink">Rating over time</h3>
                </div>
                <span className="pv-admin-chip">
                  {formatNumber(data.rating_series.length)} votes
                </span>
              </div>
              <div className="mt-2">
                <TrendLine
                  points={data.rating_series.map((point) => ({
                    x: new Date(point.at).getTime(),
                    y: point.rating,
                    label: new Date(point.at).toLocaleDateString(),
                  }))}
                  height={150}
                  formatValue={(value) => value.toFixed(0)}
                />
              </div>
              <p className="mt-1 text-[11px] leading-relaxed text-muted">
                The line ends at the current rating. Admin rebuilds and undo can
                rewrite the ledger, so history is approximate.
              </p>
            </div>

            <div>
              <div className="mb-2 flex items-end justify-between gap-2">
                <div>
                  <p className="pv-admin-label">Every duel shown, newest first</p>
                  <h3 className="font-serif text-xl text-ink">Full duel history</h3>
                </div>
                <span className="pv-admin-chip">
                  {formatNumber(data.history.total)} duels
                </span>
              </div>
              <DuelRows duels={data.history.duels} />
              {data.history.total > pageSize && (
                <AdminPagination
                  offset={offset}
                  limit={pageSize}
                  total={data.history.total}
                  onPageChange={setOffset}
                />
              )}
            </div>
          </div>
        ) : null}
      </section>

      {removeOpen && data && (
        <div className="fixed inset-0 z-[60] grid place-items-center bg-ink/50 p-4">
          <div
            role="dialog"
            aria-modal="true"
            aria-labelledby="remove-track-title"
            className="w-full max-w-lg rounded-2xl border-2 border-ink bg-card p-5 shadow-[6px_6px_0_0_var(--color-ink)]"
          >
            <p className="pv-admin-label text-danger">Destructive</p>
            <h3
              id="remove-track-title"
              className="mt-1 font-serif text-2xl tracking-[-0.02em]"
            >
              Remove this track?
            </h3>
            <p className="mt-2 text-sm leading-relaxed text-muted">
              <span className="font-semibold text-ink">{data.track.title}</span> by{' '}
              {data.track.artists}, currently #{data.track.current_rank}. This is
              meant for mistakes such as a duplicate entry.
            </p>
            <div className="mt-3 rounded-xl border border-danger/40 bg-danger/5 p-3 text-xs leading-relaxed text-danger">
              This deletes {formatNumber(data.record.shown_duels)} duels and{' '}
              {formatNumber(data.record.voted_duels)} votes involving this track,
              then replays the remaining vote ledger so every other song&apos;s
              rating, duels and wins are rebuilt. It cannot be undone.
            </div>
            <p className="mt-3 text-xs leading-relaxed text-muted">
              If you found a duplicate, open both copies and remove the lower-ranked
              one. If a voter currently has this song on screen, that duel can no
              longer be submitted.
            </p>
            <label className="mt-3 block">
              <span className="pv-admin-label text-[8px]">
                Type REMOVE to confirm
              </span>
              <input
                type="text"
                value={removeConfirm}
                onChange={(event) => {
                  setRemoveConfirm(event.target.value)
                  setRemoveError(null)
                }}
                autoComplete="off"
                className="pv-admin-input mt-1 font-mono text-sm uppercase"
                placeholder="REMOVE"
              />
            </label>
            {removeError && (
              <p role="alert" className="mt-2 text-xs text-danger">
                {removeError}
              </p>
            )}
            <div className="mt-4 flex flex-wrap justify-end gap-2">
              <button
                type="button"
                onClick={() => setRemoveOpen(false)}
                disabled={removing}
                className="pv-admin-button"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => void removeTrack()}
                disabled={removing || removeConfirm.trim() !== 'REMOVE'}
                className="pv-admin-button pv-admin-button-danger"
              >
                {removing ? 'Removing…' : 'Remove track'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
