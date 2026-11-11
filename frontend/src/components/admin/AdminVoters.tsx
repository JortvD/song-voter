import { Fragment, useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { useAuth } from '../../context/AuthContext'
import { usePlaylist } from '../../context/PlaylistContext'
import { ApiError, api } from '../../lib/api'
import type { AdminVoterStat, AdminVotersPage } from '../../lib/types'
import { localTimezone } from '../../lib/time'
import { AdminUserActions } from '../AdminUserActions'
import { AdminCrowd } from './AdminCrowd'
import { AdminVoterDetail } from './AdminVoterDetail'
import {
  AsOf,
  formatNumber,
  formatPercent,
  formatSeconds,
  formatWhen,
} from './AdminStatsUI'

const PAGE_SIZE = 25

const SORT_COLUMNS: Array<{ key: string; label: string; className?: string }> = [
  { key: 'username', label: 'Voter' },
  { key: 'votes', label: 'Votes', className: 'text-right' },
  { key: 'no_vote', label: 'No vote', className: 'text-right' },
  { key: 'median_decision', label: 'Median', className: 'text-right' },
  { key: 'agreement', label: 'Agreement', className: 'text-right' },
  { key: 'sessions', label: 'Sessions', className: 'text-right' },
  { key: 'last_active', label: 'Last active', className: 'text-right' },
  { key: 'flags', label: 'Flags', className: 'text-right' },
]

const MORE_SORT_COLUMNS: Array<{ key: string; label: string }> = [
  { key: 'p10', label: 'P10' },
  { key: 'p90', label: 'P90' },
  { key: 'fast_share', label: '< fast' },
  { key: 'side_bias', label: 'Side bias' },
  { key: 'avg_session', label: 'Avg / session' },
  { key: 'voting_time', label: 'Voting time' },
  { key: 'songs', label: 'Songs seen' },
]

function FlagBadges({ voter }: { voter: AdminVoterStat }) {
  if (!voter.flags.fast_clicker && !voter.flags.side_bias && !voter.flags.low_agreement) {
    return <span className="text-muted">·</span>
  }
  return (
    <span className="flex flex-wrap justify-end gap-1">
      {voter.flags.fast_clicker && (
        <span
          title="Median decision time is below the fast-decision threshold."
          className="rounded-full border border-danger/40 bg-danger/5 px-1.5 py-0.5 font-mono text-[7px] uppercase tracking-[0.1em] text-danger"
        >
          fast
        </span>
      )}
      {voter.flags.side_bias && (
        <span
          title="Left-side picks are outside the unbiased band. Informational only."
          className="rounded-full border border-danger/40 bg-danger/5 px-1.5 py-0.5 font-mono text-[7px] uppercase tracking-[0.1em] text-danger"
        >
          side
        </span>
      )}
      {voter.flags.low_agreement && (
        <span
          title="Agreement with the current standings is below the threshold with enough votes."
          className="rounded-full border border-danger/40 bg-danger/5 px-1.5 py-0.5 font-mono text-[7px] uppercase tracking-[0.1em] text-danger"
        >
          low agree
        </span>
      )}
    </span>
  )
}

function SortHeader({
  column,
  sort,
  onSort,
}: {
  column: { key: string; label: string; className?: string }
  sort: string
  onSort: (key: string) => void
}) {
  return (
    <th className={column.className}>
      <button
        type="button"
        onClick={() => onSort(column.key)}
        className={`inline-flex items-center gap-1 font-mono text-[8px] uppercase tracking-[0.14em] transition hover:text-ink ${
          sort === column.key ? 'text-ink' : 'text-muted'
        }`}
      >
        {column.label}
        {sort === column.key && <span>↓</span>}
      </button>
    </th>
  )
}

export function AdminVoters() {
  const { user: currentUser } = useAuth()
  const { refresh: refreshPlaylist } = usePlaylist()
  const [data, setData] = useState<AdminVotersPage | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState('last_active')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [detailId, setDetailId] = useState<string | null>(null)
  const [moreColumns, setMoreColumns] = useState(false)
  const timezone = localTimezone()

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getAdminVoters(offset, PAGE_SIZE, query, sort)
      .then((next) => {
        if (active) setData(next)
      })
      .catch((caught: unknown) => {
        if (active) {
          setError(caught instanceof ApiError ? caught.message : 'Could not load voters')
        }
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
    }
  }, [offset, query, refreshKey, sort])

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

  function handleChanged(message: string) {
    setNotice(message)
    setRefreshKey((value) => value + 1)
    void refreshPlaylist()
  }

  function handleDeleted(message: string) {
    setNotice(message)
    setExpandedId(null)
    void refreshPlaylist()
    if (data && data.voters.length === 1 && offset > 0) {
      setOffset(Math.max(0, offset - PAGE_SIZE))
    } else {
      setRefreshKey((value) => value + 1)
    }
  }

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0
  const flagByKey = new Map((data?.flag_summary ?? []).map((flag) => [flag.key, flag]))

  return (
    <div className="space-y-4">
      <section className="panel">
        <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line px-5 py-3">
          <div>
            <h2 className="label">Voter behaviour</h2>
            <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {data
                ? `${formatNumber(data.total)} voters · ${formatNumber(data.flagged_voters)} flagged`
                : error
                  ? 'Unavailable'
                  : 'Loading…'}
            </p>
            <AsOf value={data?.as_of} />
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <a href="/api/admin/users.csv" className="pv-admin-button">
              Export voters CSV
            </a>
            <button
              type="button"
              onClick={() => setMoreColumns((value) => !value)}
              aria-pressed={moreColumns}
              className="pv-admin-button"
            >
              {moreColumns ? 'Fewer columns' : 'More columns'}
            </button>
            <form onSubmit={applySearch} className="flex flex-wrap gap-2">
              <input
                type="search"
                value={queryInput}
                onChange={(event) => setQueryInput(event.target.value)}
                placeholder="Search username"
                className="field w-44 py-1.5 text-xs"
                aria-label="Search voters"
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

        <div className="grid gap-2 px-5 pt-4 sm:grid-cols-2 xl:grid-cols-4">
          {(['fast_clicker', 'side_bias', 'low_agreement'] as const).map((key) => {
            const flag = flagByKey.get(key)
            const label =
              key === 'fast_clicker'
                ? 'Fast clickers'
                : key === 'side_bias'
                  ? 'Side-biased'
                  : 'Low agreement'
            return (
              <div key={key} className="rounded-xl border border-line bg-paper p-3">
                <p className="pv-admin-label text-[8px]">{label}</p>
                <p className="mt-1 font-serif text-2xl leading-none">
                  {formatNumber(flag?.voters ?? 0)}
                </p>
                <p className="mt-1 text-[11px] leading-snug text-muted">
                  {formatPercent(flag?.vote_share_pct ?? 0, 1)} of all votes
                </p>
              </div>
            )
          })}
          <div className="rounded-xl border border-line bg-paper p-3">
            <p className="pv-admin-label text-[8px]">Any flag</p>
            <p className="mt-1 font-serif text-2xl leading-none">
              {formatNumber(data?.flagged_voters ?? 0)}
            </p>
            <p className="mt-1 text-[11px] leading-snug text-muted">
              {formatPercent(data?.flagged_vote_share_pct ?? 0, 1)} of all votes
            </p>
          </div>
        </div>

        <div className="p-5">
          {notice && (
            <p className="mb-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
              {notice}
            </p>
          )}

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
          ) : data && data.voters.length > 0 ? (
            <>
              <p className="mb-2 text-[11px] leading-relaxed text-muted">
                Flags are informational only and are never shown to voters. The
                no-vote rate includes skips, undone votes and a duel currently on
                screen, so it is an upper bound on the skip rate.
              </p>
              <div className="overflow-x-auto">
                <table
                  className={`w-full table-fixed text-left text-sm ${
                    moreColumns ? 'min-w-[1120px]' : 'min-w-[820px]'
                  }`}
                >
                  <thead>
                    <tr className="border-b border-line">
                      <SortHeader column={SORT_COLUMNS[0]} sort={sort} onSort={setSort} />
                      {SORT_COLUMNS.slice(1).map((column) => (
                        <SortHeader key={column.key} column={column} sort={sort} onSort={setSort} />
                      ))}
                      {moreColumns &&
                        MORE_SORT_COLUMNS.map((column) => (
                          <SortHeader
                            key={column.key}
                            column={{ ...column, className: 'text-right' }}
                            sort={sort}
                            onSort={setSort}
                          />
                        ))}
                      <th className="w-24 text-right">Manage</th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.voters.map((voter) => {
                      const isSelf = currentUser?.id === voter.id
                      const isExpanded = expandedId === voter.id
                      const userRow = {
                        id: voter.id,
                        username: voter.username,
                        is_admin: voter.is_admin,
                        disabled_at: voter.disabled_at,
                        created_at: voter.created_at,
                        votes_cast: voter.votes,
                        matchups_seen: voter.duels,
                        last_vote_at: null,
                        last_seen_at: null,
                        last_active_at: voter.last_active_at,
                      }

                      return (
                        <Fragment key={voter.id}>
                          <tr className="border-b border-line align-top">
                            <td className="px-2 py-2.5">
                              <span className="flex min-w-0 flex-wrap items-center gap-1.5">
                                <button
                                  type="button"
                                  onClick={() => setDetailId(voter.id)}
                                  className="max-w-full truncate font-medium underline decoration-transparent underline-offset-2 hover:decoration-line"
                                >
                                  {voter.username}
                                </button>
                                {isSelf && <span className="pv-admin-chip">You</span>}
                                {voter.is_admin && (
                                  <span className="pv-admin-chip border-cobalt/40 text-cobalt">
                                    Admin
                                  </span>
                                )}
                                {voter.disabled_at && (
                                  <span className="pv-admin-chip border-danger/40 text-danger">
                                    Disabled
                                  </span>
                                )}
                              </span>
                            </td>
                            <td className="px-2 py-2.5 text-right font-mono text-xs">
                              {formatNumber(voter.votes)}
                            </td>
                            <td className="px-2 py-2.5 text-right font-mono text-xs">
                              {formatPercent(voter.no_vote_rate, 1)}
                            </td>
                            <td className="px-2 py-2.5 text-right font-mono text-xs">
                              {formatSeconds(voter.median_decision_seconds)}
                            </td>
                            <td className="px-2 py-2.5 text-right font-mono text-xs">
                              {formatPercent(voter.agreement_pct, 1)}
                            </td>
                            <td className="px-2 py-2.5 text-right font-mono text-xs">
                              {formatNumber(voter.sessions)}
                            </td>
                            <td className="whitespace-nowrap px-2 py-2.5 text-right font-mono text-[9px] text-muted">
                              {formatWhen(voter.last_active_at)}
                            </td>
                            <td className="px-2 py-2.5 text-right">
                              <FlagBadges voter={voter} />
                            </td>
                            {moreColumns && (
                              <>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatSeconds(voter.p10_decision_seconds)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatSeconds(voter.p90_decision_seconds)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatPercent(voter.fast_share_pct, 1)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatPercent(voter.side_bias_pct, 1)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {voter.avg_duels_per_session.toFixed(1)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatSeconds(voter.total_voting_time_seconds)}
                                </td>
                                <td className="px-2 py-2.5 text-right font-mono text-[10px]">
                                  {formatNumber(voter.distinct_songs_seen)}
                                </td>
                              </>
                            )}
                            <td className="px-2 py-2.5 text-right">
                              <button
                                type="button"
                                onClick={() => setExpandedId(isExpanded ? null : voter.id)}
                                aria-expanded={isExpanded}
                                className="rounded-full border border-line px-2 py-1 font-mono text-[8px] uppercase tracking-[0.1em] text-muted transition hover:border-ink hover:text-ink"
                              >
                                {isExpanded ? 'Close' : 'Manage'}
                              </button>
                            </td>
                          </tr>

                          {isExpanded && (
                            <tr className="border-b border-line">
                              <td
                                colSpan={moreColumns ? 16 : 9}
                                className="p-0"
                              >
                                <AdminUserActions
                                  user={userRow}
                                  isSelf={isSelf}
                                  onChanged={handleChanged}
                                  onDeleted={handleDeleted}
                                />
                              </td>
                            </tr>
                          )}
                        </Fragment>
                      )
                    })}
                  </tbody>
                </table>
              </div>

              <div className="mt-4 flex items-center justify-between gap-3">
                <button
                  type="button"
                  onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
                  disabled={offset === 0}
                  className="btn-ghost"
                >
                  Previous
                </button>
                <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                  {first}–{last} of {formatNumber(data.total)}
                </span>
                <button
                  type="button"
                  onClick={() => setOffset(offset + PAGE_SIZE)}
                  disabled={last >= data.total}
                  className="btn-ghost"
                >
                  Next
                </button>
              </div>
            </>
          ) : (
            <p className="border border-dashed border-line px-3 py-6 text-center text-xs text-muted">
              {query ? `No voters match “${query}”.` : 'No voters yet.'}
            </p>
          )}
        </div>
      </section>

      <AdminCrowd />

      {detailId && (
        <AdminVoterDetail
          userId={detailId}
          timezone={timezone}
          onClose={() => setDetailId(null)}
        />
      )}
    </div>
  )
}
