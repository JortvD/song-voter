import { Fragment, useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { useAuth } from '../context/AuthContext'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { AdminUsersPage } from '../lib/types'
import { AdminUserActions } from './AdminUserActions'

const PAGE_SIZE = 50

function formatDate(value: string) {
  return new Date(value).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  })
}

function formatDateTime(value: string) {
  return new Date(value).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

export function AdminUsers() {
  const { user: currentUser } = useAuth()
  const { refresh: refreshPlaylist } = usePlaylist()
  const [data, setData] = useState<AdminUsersPage | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [sort, setSort] = useState('last_active')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [expandedId, setExpandedId] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getAdminUsers(offset, PAGE_SIZE, query, sort)
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

    if (data && data.users.length === 1 && offset > 0) {
      setOffset(Math.max(0, offset - PAGE_SIZE))
    } else {
      setRefreshKey((value) => value + 1)
    }
  }

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0

  return (
    <section className="panel">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line px-5 py-3">
        <div>
          <h2 className="label">Voters</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {data
              ? `${formatNumber(data.total)} registered`
              : error
                ? 'Unavailable'
                : 'Loading…'}
          </p>
        </div>

        <form onSubmit={applySearch} className="flex flex-wrap gap-2">
          <input
            type="search"
            value={queryInput}
            onChange={(event) => setQueryInput(event.target.value)}
            placeholder="Search username"
            className="field w-44 py-1.5 text-xs"
            aria-label="Search voters"
          />
          <select
            value={sort}
            onChange={(event) => {
              setSort(event.target.value)
              setOffset(0)
            }}
            className="field w-36 py-1.5 text-xs"
            aria-label="Sort voters"
          >
            <option value="last_active">Last active</option>
            <option value="picks">Most picks</option>
            <option value="duels">Most duels</option>
            <option value="joined">Newest</option>
            <option value="username">Username</option>
          </select>
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

      <div className="p-5">
        {notice && (
          <p className="mb-4 border-l-2 border-ok bg-ok/5 px-3 py-2 text-xs text-ok">
            {notice}
          </p>
        )}

        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2, 3].map((index) => (
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
        ) : data && data.users.length > 0 ? (
          <>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[720px] table-fixed text-left text-sm">
                <thead>
                  <tr className="border-b border-line font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    <th className="w-[32%] px-2 pb-2 font-medium">Voter</th>
                    <th className="w-16 px-2 pb-2 text-right font-medium">Picks</th>
                    <th className="hidden w-16 px-2 pb-2 text-right font-medium sm:table-cell">
                      Duels
                    </th>
                    <th className="hidden w-28 px-2 pb-2 font-medium md:table-cell">
                      Joined
                    </th>
                    <th className="w-32 px-2 pb-2 text-right font-medium">Last active</th>
                    <th className="w-20 px-2 pb-2 text-right font-medium">Manage</th>
                  </tr>
                </thead>
                <tbody>
                  {data.users.map((user) => {
                    const isSelf = currentUser?.id === user.id
                    const isExpanded = expandedId === user.id

                    return (
                      <Fragment key={user.id}>
                        <tr className="border-b border-line last:border-b-0">
                          <td className="px-2 py-2.5">
                            <span className="flex min-w-0 flex-wrap items-center gap-2">
                              <span className="truncate font-medium">{user.username}</span>
                              {isSelf && (
                                <span className="shrink-0 border border-ink/30 px-1 py-0.5 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                                  You
                                </span>
                              )}
                              {user.is_admin && (
                                <span className="shrink-0 border border-cobalt/40 px-1 py-0.5 font-mono text-[9px] uppercase tracking-[0.12em] text-cobalt">
                                  Admin
                                </span>
                              )}
                              {user.disabled_at && (
                                <span className="shrink-0 border border-danger/40 px-1 py-0.5 font-mono text-[9px] uppercase tracking-[0.12em] text-danger">
                                  Disabled
                                </span>
                              )}
                            </span>
                          </td>
                          <td className="px-2 py-2.5 text-right font-mono text-xs">
                            {formatNumber(user.votes_cast)}
                          </td>
                          <td className="hidden px-2 py-2.5 text-right font-mono text-xs sm:table-cell">
                            {formatNumber(user.matchups_seen)}
                          </td>
                          <td className="hidden px-2 py-2.5 font-mono text-[10px] text-muted md:table-cell">
                            {formatDate(user.created_at)}
                          </td>
                          <td className="whitespace-nowrap px-2 py-2.5 text-right font-mono text-[10px] text-muted">
                            {formatDateTime(user.last_active_at)}
                          </td>
                          <td className="px-2 py-2.5 text-right">
                            <button
                              type="button"
                              onClick={() =>
                                setExpandedId(isExpanded ? null : user.id)
                              }
                              aria-expanded={isExpanded}
                              className="border border-line px-2 py-1 font-mono text-[9px] uppercase tracking-[0.1em] text-muted transition hover:border-ink hover:text-ink"
                            >
                              {isExpanded ? 'Close' : 'Manage'}
                            </button>
                          </td>
                        </tr>

                        {isExpanded && (
                          <tr className="border-b border-line">
                            <td colSpan={6} className="p-0">
                              <AdminUserActions
                                user={user}
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
  )
}
