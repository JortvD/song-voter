import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'

import { ApiError, api } from '../lib/api'
import type { AdminUsersPage } from '../lib/types'

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
  const [data, setData] = useState<AdminUsersPage | null>(null)
  const [offset, setOffset] = useState(0)
  const [queryInput, setQueryInput] = useState('')
  const [query, setQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setLoading(true)
    setError(null)

    api
      .getAdminUsers(offset, PAGE_SIZE, query)
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
  }, [offset, query])

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

  const first = data && data.total > 0 ? data.offset + 1 : 0
  const last = data ? Math.min(data.offset + data.limit, data.total) : 0

  return (
    <section className="panel">
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line px-5 py-3">
        <div>
          <h2 className="label">Voters</h2>
          <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {data ? `${formatNumber(data.total)} registered` : 'Loading…'}
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
        {error && (
          <p className="mb-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-xs text-accent">
            {error}
          </p>
        )}

        {loading ? (
          <div className="space-y-2">
            {[0, 1, 2, 3].map((index) => (
              <div key={index} className="h-10 animate-pulse border border-line bg-paper" />
            ))}
          </div>
        ) : data && data.users.length > 0 ? (
          <>
            <div className="overflow-x-auto">
              <table className="w-full min-w-[560px] text-left text-sm">
                <thead>
                  <tr className="border-b border-line font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
                    <th className="pb-2 font-medium">Voter</th>
                    <th className="pb-2 text-right font-medium">Picks</th>
                    <th className="hidden pb-2 text-right font-medium sm:table-cell">
                      Duels
                    </th>
                    <th className="hidden pb-2 font-medium md:table-cell">Joined</th>
                    <th className="pb-2 text-right font-medium">Last active</th>
                  </tr>
                </thead>
                <tbody>
                  {data.users.map((user) => (
                    <tr key={user.id} className="border-b border-line last:border-b-0">
                      <td className="py-2.5">
                        <span className="flex items-center gap-2">
                          <span className="font-medium">{user.username}</span>
                          {user.is_admin && (
                            <span className="border border-accent/40 px-1 py-0.5 font-mono text-[8px] uppercase tracking-[0.12em] text-accent">
                              Admin
                            </span>
                          )}
                        </span>
                      </td>
                      <td className="py-2.5 text-right font-mono text-xs">
                        {formatNumber(user.votes_cast)}
                      </td>
                      <td className="hidden py-2.5 text-right font-mono text-xs sm:table-cell">
                        {formatNumber(user.matchups_seen)}
                      </td>
                      <td className="hidden py-2.5 font-mono text-[10px] text-muted md:table-cell">
                        {formatDate(user.created_at)}
                      </td>
                      <td className="py-2.5 text-right font-mono text-[10px] text-muted">
                        {formatDateTime(user.last_active_at)}
                      </td>
                    </tr>
                  ))}
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
