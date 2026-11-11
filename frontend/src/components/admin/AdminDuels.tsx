import { Fragment, useCallback, useEffect, useState } from 'react'

import { ApiError, api } from '../../lib/api'
import type { AdminDuel, AdminDuelPage } from '../../lib/types'
import {
  AsOf,
  formatDecimal,
  formatNumber,
  formatPercent,
  formatSeconds,
  formatWhen,
} from './AdminStatsUI'

function ResultBadge({ duel }: { duel: AdminDuel }) {
  if (duel.no_vote) {
    return (
      <span className="inline-flex items-center gap-1 rounded-full border border-line bg-paper px-2 py-0.5 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
        —
      </span>
    )
  }

  return (
    <span className="flex flex-wrap items-center gap-1">
      <span
        className={`inline-flex size-6 items-center justify-center rounded-full border-2 font-mono text-[10px] font-bold ${
          duel.result_side === 'A'
            ? 'border-ink bg-butter text-ink'
            : 'border-ink bg-cobalt text-paper'
        }`}
      >
        {duel.result_side}
      </span>
      {duel.upset && (
        <span className="rounded-full border border-danger/40 bg-danger/10 px-1.5 py-0.5 font-mono text-[7px] font-semibold uppercase tracking-[0.1em] text-danger">
          upset
        </span>
      )}
    </span>
  )
}

function DuelTrackCell({
  track,
  side,
  onOpenSong,
}: {
  track: AdminDuel['track_a']
  side: 'A' | 'B'
  onOpenSong?: (id: string) => void
}) {
  return (
    <span className="flex min-w-0 items-center gap-2">
      <span
        className={`inline-flex size-5 shrink-0 items-center justify-center rounded-md border font-mono text-[8px] font-bold ${
          side === 'A' ? 'border-ink/40 bg-butter/50' : 'border-ink/40 bg-cobalt/10'
        }`}
      >
        {side}
      </span>
      <span className="min-w-0">
        <button
          type="button"
          onClick={(event) => {
            event.stopPropagation()
            onOpenSong?.(track.id)
          }}
          className="block max-w-full truncate text-left text-xs font-medium text-ink underline decoration-transparent underline-offset-2 hover:decoration-line"
          title={track.title}
        >
          {track.title}
        </button>
        <span className="block truncate font-mono text-[8px] uppercase tracking-[0.08em] text-muted">
          #{track.current_rank} · {formatNumber(track.current_rating)}
        </span>
      </span>
    </span>
  )
}

export function DuelDetailsGrid({ duel }: { duel: AdminDuel }) {
  const { details } = duel
  const rows: Array<[string, string]> = [
    [
      'Artists / years',
      `${duel.track_a.artists}${duel.track_a.release_year ? ` · ${duel.track_a.release_year}` : ''} — ${duel.track_b.artists}${duel.track_b.release_year ? ` · ${duel.track_b.release_year}` : ''}`,
    ],
    ['Side shown', details.served_flip ? 'A = higher-id track' : 'A = lower-id track'],
    [
      'Ratings before',
      `A ${formatDecimal(details.rating_a_before)} · B ${formatDecimal(details.rating_b_before)}`,
    ],
    [
      'Win chance before',
      `A ${formatPercent(details.win_chance_a, 1)} · B ${formatPercent(details.win_chance_b, 1)}`,
    ],
    ['Rating gap', formatDecimal(details.rating_gap_before)],
    [
      'Rating change',
      `A ${details.delta_a == null ? '—' : `${details.delta_a >= 0 ? '+' : ''}${details.delta_a.toFixed(1)}`} · B ${details.delta_b == null ? '—' : `${details.delta_b >= 0 ? '+' : ''}${details.delta_b.toFixed(1)}`}`,
    ],
    [
      'Duels before',
      `A ${formatNumber(details.comparisons_a_before)} · B ${formatNumber(details.comparisons_b_before)}`,
    ],
    ['K factor', formatDecimal(details.k_factor, 1)],
    [
      'Current ratings',
      `A ${formatDecimal(duel.track_a.current_rating)} · B ${formatDecimal(duel.track_b.current_rating)}`,
    ],
    ['Vote sequence', details.vote_seq == null ? '—' : `#${details.vote_seq}`],
  ]

  return (
    <div className="grid gap-x-6 gap-y-1.5 rounded-xl border border-line bg-paper p-3 sm:grid-cols-2">
      {rows.map(([label, value]) => (
        <div key={label} className="flex min-w-0 items-baseline justify-between gap-3">
          <span className="shrink-0 font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
            {label}
          </span>
          <span className="min-w-0 truncate text-right text-[11px] text-ink" title={value}>
            {value}
          </span>
        </div>
      ))}
      {duel.idle && (
        <p className="sm:col-span-2 font-mono text-[8px] uppercase tracking-[0.1em] text-danger">
          Idle decision — excluded from decision-time statistics.
        </p>
      )}
    </div>
  )
}

export function DuelRows({
  duels,
  onOpenSong,
}: {
  duels: AdminDuel[]
  onOpenSong?: (id: string) => void
}) {
  const [expanded, setExpanded] = useState<string | null>(null)

  if (duels.length === 0) {
    return (
      <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
        No duels match these filters.
      </p>
    )
  }

  return (
    <div className="overflow-x-auto">
      <table className="pv-admin-table min-w-[780px] table-fixed">
        <thead>
          <tr>
            <th className="w-[104px]">Time</th>
            <th className="w-[92px]">Voter</th>
            <th className="w-[190px]">Song A · left</th>
            <th className="w-[190px]">Song B · right</th>
            <th className="w-[76px]">Result</th>
            <th className="w-[58px] text-right">Chance</th>
            <th className="w-[54px] text-right">Swing</th>
            <th className="w-[54px] text-right">Secs</th>
            <th className="w-[58px]" />
          </tr>
        </thead>
        <tbody>
          {duels.map((duel) => {
            const key = `${duel.user_id}-${duel.seen_at}-${duel.track_a.id}-${duel.track_b.id}`
            const isExpanded = expanded === key
            return (
              <Fragment key={key}>
                <tr className={duel.no_vote ? 'bg-paper/40' : undefined}>
                  <td className="whitespace-nowrap font-mono text-[9px] text-muted">
                    {formatWhen(duel.seen_at)}
                  </td>
                  <td className="truncate text-xs font-medium text-ink">{duel.username}</td>
                  <td>
                    <DuelTrackCell track={duel.track_a} side="A" onOpenSong={onOpenSong} />
                  </td>
                  <td>
                    <DuelTrackCell track={duel.track_b} side="B" onOpenSong={onOpenSong} />
                  </td>
                  <td>
                    <ResultBadge duel={duel} />
                  </td>
                  <td className="text-right font-mono text-[10px] text-ink">
                    {duel.no_vote ? '—' : formatPercent(duel.win_chance_pct)}
                  </td>
                  <td className="text-right font-mono text-[10px] text-muted">
                    {duel.rating_swing == null
                      ? '—'
                      : `${duel.rating_swing >= 0 ? '+' : ''}${duel.rating_swing.toFixed(1)}`}
                  </td>
                  <td className="text-right font-mono text-[10px] text-muted">
                    {duel.no_vote ? '—' : duel.idle ? 'idle' : formatSeconds(duel.decision_seconds)}
                  </td>
                  <td className="text-right">
                    <button
                      type="button"
                      onClick={() => setExpanded(isExpanded ? null : key)}
                      aria-expanded={isExpanded}
                      className="rounded-full border border-line px-2 py-1 font-mono text-[8px] uppercase tracking-[0.1em] text-muted transition hover:border-ink hover:text-ink"
                    >
                      {isExpanded ? 'Less' : 'More'}
                    </button>
                  </td>
                </tr>
                {isExpanded && (
                  <tr>
                    <td colSpan={9} className="bg-paper/30">
                      <DuelDetailsGrid duel={duel} />
                    </td>
                  </tr>
                )}
              </Fragment>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

const PAGE_SIZES = [15, 50, 100]
const ONLY_OPTIONS = [
  { value: 'all', label: 'All duels' },
  { value: 'voted', label: 'Only voted' },
  { value: 'no_vote', label: 'Only no vote' },
  { value: 'upset', label: 'Only upsets' },
] as const
const TIME_OPTIONS = [
  { value: '', label: 'Any time' },
  { value: '1', label: 'Last hour' },
  { value: '6', label: 'Last 6h' },
  { value: '24', label: 'Last 24h' },
  { value: '168', label: 'Last 7d' },
  { value: '720', label: 'Last 30d' },
] as const

export function AdminDuelFeed({
  title = 'Duel feed',
  initialUserId,
  initialTrackId,
  initialOnly = 'all',
  showFilters = true,
  onOpenSong,
}: {
  title?: string
  initialUserId?: string
  initialTrackId?: string
  initialOnly?: 'all' | 'voted' | 'no_vote' | 'upset'
  showFilters?: boolean
  onOpenSong?: (id: string) => void
}) {
  const [data, setData] = useState<AdminDuelPage | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [offset, setOffset] = useState(0)
  const [pageSize, setPageSize] = useState(15)
  const [only, setOnly] = useState(initialOnly)
  const [hours, setHours] = useState('')
  const [subject, setSubject] = useState('')
  const [voter, setVoter] = useState('')
  const [from, setFrom] = useState('')
  const [to, setTo] = useState('')
  const [applied, setApplied] = useState({
    only: initialOnly,
    hours: '',
    subject: '',
    voter: '',
    from: '',
    to: '',
  })

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const next = await api.getAdminDuels({
        offset,
        limit: pageSize,
        only: applied.only,
        userId: initialUserId,
        trackId: initialTrackId,
        q: applied.subject,
        user: applied.voter,
        hours: applied.hours ? Number(applied.hours) : undefined,
        from: applied.from ? new Date(applied.from).toISOString() : undefined,
        to: applied.to ? new Date(applied.to).toISOString() : undefined,
      })
      setData(next)
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load duels')
    } finally {
      setLoading(false)
    }
  }, [
    applied,
    initialTrackId,
    initialUserId,
    offset,
    pageSize,
  ])

  useEffect(() => {
    void load()
  }, [load])

  function applyFilters() {
    setOffset(0)
    setApplied({ only, hours, subject, voter, from, to })
  }

  function clearFilters() {
    setOnly(initialOnly)
    setHours('')
    setSubject('')
    setVoter('')
    setFrom('')
    setTo('')
    setOffset(0)
    setApplied({ only: initialOnly, hours: '', subject: '', voter: '', from: '', to: '' })
  }

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Newest first</p>
          <h3 className="font-serif text-xl text-ink">{title}</h3>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <AsOf value={data?.as_of} />
          <span className="pv-admin-chip">{formatNumber(data?.total ?? 0)} rows</span>
        </div>
      </div>

      {showFilters && (
        <div className="mt-3 grid gap-2 rounded-xl border border-line bg-paper p-3 md:grid-cols-3 xl:grid-cols-6">
          <label className="block">
            <span className="pv-admin-label text-[8px]">Show</span>
            <select
              value={only}
              onChange={(event) => setOnly(event.target.value as typeof only)}
              className="pv-admin-input mt-1 py-1.5 text-xs"
            >
              {ONLY_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="pv-admin-label text-[8px]">Page size</span>
            <select
              value={pageSize}
              onChange={(event) => {
                setPageSize(Number(event.target.value))
                setOffset(0)
              }}
              className="pv-admin-input mt-1 py-1.5 text-xs"
            >
              {PAGE_SIZES.map((size) => (
                <option key={size} value={size}>
                  {size} rows
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="pv-admin-label text-[8px]">Window</span>
            <select
              value={hours}
              onChange={(event) => setHours(event.target.value)}
              className="pv-admin-input mt-1 py-1.5 text-xs"
            >
              {TIME_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span className="pv-admin-label text-[8px]">Voter</span>
            <input
              type="search"
              value={voter}
              onChange={(event) => setVoter(event.target.value)}
              placeholder="username"
              className="pv-admin-input mt-1 py-1.5 text-xs"
            />
          </label>
          <label className="block">
            <span className="pv-admin-label text-[8px]">Song title</span>
            <input
              type="search"
              value={subject}
              onChange={(event) => setSubject(event.target.value)}
              placeholder="either side"
              className="pv-admin-input mt-1 py-1.5 text-xs"
            />
          </label>
          <div className="flex items-end gap-2">
            <button type="button" onClick={applyFilters} className="pv-admin-button flex-1">
              Apply
            </button>
            <button type="button" onClick={clearFilters} className="pv-admin-button">
              Clear
            </button>
          </div>
          <label className="block md:col-span-3">
            <span className="pv-admin-label text-[8px]">Custom from</span>
            <input
              type="datetime-local"
              value={from}
              onChange={(event) => setFrom(event.target.value)}
              className="pv-admin-input mt-1 py-1.5 text-xs"
            />
          </label>
          <label className="block md:col-span-3">
            <span className="pv-admin-label text-[8px]">Custom to</span>
            <input
              type="datetime-local"
              value={to}
              onChange={(event) => setTo(event.target.value)}
              className="pv-admin-input mt-1 py-1.5 text-xs"
            />
          </label>
        </div>
      )}

      {error && <p className="mt-3 text-sm text-danger">{error}</p>}

      <div className="mt-3">
        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2, 3, 4].map((index) => (
              <div key={index} className="h-10 animate-pulse rounded-xl bg-ink/10" />
            ))}
          </div>
        ) : data ? (
          <DuelRows duels={data.duels} onOpenSong={onOpenSong} />
        ) : null}
      </div>

      {data && data.total > 0 && (
        <div className="mt-3 flex items-center justify-between gap-3">
          <button
            type="button"
            onClick={() => setOffset(Math.max(0, offset - pageSize))}
            disabled={offset === 0}
            className="pv-admin-button"
          >
            Previous
          </button>
          <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {first}–{last} of {formatNumber(data.total)}
          </span>
          <button
            type="button"
            onClick={() => setOffset(offset + pageSize)}
            disabled={last >= data.total}
            className="pv-admin-button"
          >
            Next
          </button>
        </div>
      )}
    </div>
  )
}

const UPSET_WINDOWS = [
  { hours: 1, label: 'Last hour' },
  { hours: 6, label: 'Last 6h' },
  { hours: 24, label: 'Last 24h' },
  { hours: 168, label: 'Last 7d' },
]

export function BiggestUpsets({ onOpenSong }: { onOpenSong?: (id: string) => void }) {
  const [hours, setHours] = useState(24)
  const [data, setData] = useState<AdminDuelPage | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)
    api
      .getAdminDuels({ only: 'upset', sort: 'upsets', hours, limit: 8 })
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load upsets')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })
    return () => {
      active = false
    }
  }, [hours])

  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Lowest winner chance first</p>
          <h3 className="font-serif text-xl text-ink">Biggest upsets</h3>
        </div>
        <select
          value={hours}
          onChange={(event) => setHours(Number(event.target.value))}
          className="pv-admin-input w-auto py-1.5 text-xs"
          aria-label="Upset window"
        >
          {UPSET_WINDOWS.map((window) => (
            <option key={window.hours} value={window.hours}>
              {window.label}
            </option>
          ))}
        </select>
      </div>
      {error && <p className="mt-3 text-sm text-danger">{error}</p>}
      {loading ? (
        <div className="mt-3 space-y-2">
          {[0, 1, 2].map((index) => (
            <div key={index} className="h-12 animate-pulse rounded-xl bg-ink/10" />
          ))}
        </div>
      ) : data && data.duels.length > 0 ? (
        <ol className="mt-3 space-y-2">
          {data.duels.map((duel) => (
            <li
              key={`${duel.user_id}-${duel.seen_at}-${duel.track_a.id}`}
              className="rounded-xl border border-line bg-paper p-2.5"
            >
              <div className="flex items-center justify-between gap-2">
                <span className="pv-admin-chip border-danger/30 text-danger">
                  {formatPercent(duel.win_chance_pct, 1)} chance
                </span>
                <span className="font-mono text-[8px] uppercase tracking-[0.1em] text-muted">
                  {formatWhen(duel.seen_at)} · {duel.username}
                </span>
              </div>
              <p className="mt-1.5 text-xs leading-relaxed text-muted">
                <button
                  type="button"
                  onClick={() => onOpenSong?.(duel.winner_id ?? '')}
                  className="font-semibold text-ink underline decoration-transparent underline-offset-2 hover:decoration-line"
                >
                  {duel.result_side === 'A' ? duel.track_a.title : duel.track_b.title}
                </button>{' '}
                beat{' '}
                <button
                  type="button"
                  onClick={() =>
                    onOpenSong?.(
                      duel.result_side === 'A' ? duel.track_b.id : duel.track_a.id,
                    )
                  }
                  className="underline decoration-transparent underline-offset-2 hover:decoration-line"
                >
                  {duel.result_side === 'A' ? duel.track_b.title : duel.track_a.title}
                </button>
              </p>
            </li>
          ))}
        </ol>
      ) : (
        <p className="mt-3 text-sm text-muted">No upsets in this window.</p>
      )}
    </div>
  )
}
