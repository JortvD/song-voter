import { useCallback, useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { AdminDangerZone } from '../components/AdminDangerZone'
import { AdminInvites } from '../components/AdminInvites'
import { AdminPlaylist } from '../components/AdminPlaylist'
import { AdminRankingTable } from '../components/AdminRankingTable'
import { AdminReveal } from '../components/AdminReveal'
import { AdminUsers } from '../components/AdminUsers'
import { RequireAdmin } from '../components/RequireAdmin'
import { StatusCard } from '../components/StatusCard'
import { ApiError, api } from '../lib/api'
import type { DashboardStats } from '../lib/types'

function formatNumber(value: number) {
  return value.toLocaleString()
}

function SectionHeading({
  number,
  title,
  aside,
}: {
  number: string
  title: string
  aside?: string
}) {
  return (
    <div className="mb-3 flex items-end justify-between gap-3 border-b border-line pb-2">
      <div className="flex items-baseline gap-3">
        <span className="font-serif text-2xl leading-none text-accent">{number}</span>
        <h2 className="font-serif text-2xl leading-none">{title}</h2>
      </div>
      {aside && (
        <span className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
          {aside}
        </span>
      )}
    </div>
  )
}

function StatCard({
  label,
  value,
  hint,
}: {
  label: string
  value: string
  hint?: string
}) {
  return (
    <div className="panel p-4">
      <p className="label">{label}</p>
      <p className="mt-2 font-serif text-3xl leading-none">{value}</p>
      {hint && (
        <p className="mt-2 font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
          {hint}
        </p>
      )}
    </div>
  )
}

function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      setStats(await api.getDashboardStats())
    } catch (caught) {
      setError(caught instanceof ApiError ? caught.message : 'Could not load dashboard stats')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load, refreshKey])

  const refresh = () => setRefreshKey((value) => value + 1)

  const daily = stats?.daily ?? []
  const maxDaily = Math.max(1, ...daily.map((day) => day.votes))

  return (
    <>
      <div className="flex flex-wrap items-end justify-between gap-3 border-b border-line pb-4">
        <div>
          <p className="label">Admin</p>
          <h1 className="mt-2 font-serif text-4xl leading-none sm:text-5xl">
            The control room.
          </h1>
          <p className="mt-2 font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
            {stats?.playlist_name ?? 'No active playlist'} ·{' '}
            {stats?.revealed ? 'Standings live' : 'Standings sealed'}
          </p>
        </div>

        <div className="flex flex-wrap gap-2">
          <button type="button" onClick={refresh} disabled={loading} className="btn-ghost">
            {loading ? 'Refreshing…' : 'Refresh'}
          </button>
          {stats?.revealed && (
            <Link to="/standings" className="btn-ghost">
              Public standings
            </Link>
          )}
          <Link to="/" className="btn-ghost">
            Home
          </Link>
        </div>
      </div>

      {error && (
        <p className="mt-4 border-l-2 border-accent bg-accent/5 px-3 py-2 text-sm text-accent">
          {error}
        </p>
      )}

      <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
        <StatCard
          label="Picks"
          value={stats ? formatNumber(stats.total_votes) : '—'}
          hint={stats ? `${formatNumber(stats.votes_today)} today` : undefined}
        />
        <StatCard
          label="Duels"
          value={stats ? formatNumber(stats.total_matchups) : '—'}
          hint="pairs compared"
        />
        <StatCard
          label="Active voters"
          value={stats ? formatNumber(stats.active_users) : '—'}
          hint={stats ? `${formatNumber(stats.active_today)} today` : undefined}
        />
        <StatCard
          label="Registered"
          value={stats ? formatNumber(stats.total_users) : '—'}
          hint="accounts"
        />
        <StatCard
          label="Songs compared"
          value={
            stats
              ? `${formatNumber(stats.tracks_compared)} / ${formatNumber(stats.total_tracks)}`
              : '—'
          }
          hint="coverage"
        />
      </div>

      <div className="mt-8">
        <SectionHeading number="01" title="Activity" aside="Last 14 days" />
        <section className="panel p-5">
          <div className="flex items-baseline justify-between gap-3">
            <p className="label">Picks per day</p>
            <p className="font-mono text-[9px] uppercase tracking-[0.14em] text-muted">
              {stats ? `${formatNumber(stats.total_votes)} total` : '—'}
            </p>
          </div>

          {daily.length > 0 ? (
            <>
              <div className="mt-5 flex h-36 items-end gap-1">
                {daily.map((day) => (
                  <div
                    key={day.date}
                    title={`${day.date}: ${day.votes}`}
                    className="group flex h-full flex-1 flex-col justify-end"
                  >
                    <div
                      className="w-full bg-ink/80 transition-colors group-hover:bg-accent"
                      style={{
                        height: `${
                          day.votes === 0
                            ? 0
                            : Math.max(2, (day.votes / maxDaily) * 100)
                        }%`,
                      }}
                    />
                  </div>
                ))}
              </div>
              <div className="mt-2 flex gap-1 font-mono text-[7px] uppercase tracking-[0.1em] text-muted">
                {daily.map((day, index) => (
                  <span key={day.date} className="flex-1 text-center">
                    {index % 2 === 0 ? day.date.slice(5) : ''}
                  </span>
                ))}
              </div>
            </>
          ) : (
            <p className="mt-5 text-sm text-muted">No activity yet.</p>
          )}
        </section>
      </div>

      <div className="mt-8">
        <SectionHeading number="02" title="Runtime" aside="Live health" />
        <StatusCard />
      </div>

      <div className="mt-8">
        <SectionHeading
          number="03"
          title="Voters"
          aside={stats ? `${formatNumber(stats.total_users)} accounts` : undefined}
        />
        <AdminUsers key={`users-${refreshKey}`} />
      </div>

      <div className="mt-8">
        <SectionHeading
          number="04"
          title="Standings"
          aside={stats ? `${formatNumber(stats.total_tracks)} songs` : undefined}
        />
        <AdminRankingTable key={`ranking-${refreshKey}`} />
      </div>

      <div className="mt-8">
        <SectionHeading number="05" title="Operations" />
        <div className="grid items-start gap-6 lg:grid-cols-[1.3fr_0.7fr]">
          <div className="space-y-6">
            <AdminPlaylist key={`playlist-${refreshKey}`} />
            <AdminReveal key={`reveal-${refreshKey}`} />
          </div>
          <AdminInvites key={`invites-${refreshKey}`} />
        </div>
      </div>

      <div className="mt-8">
        <SectionHeading number="06" title="Danger zone" />
        <AdminDangerZone onReset={refresh} onRemove={refresh} />
      </div>
    </>
  )
}

export function AdminPage() {
  return (
    <main className="mx-auto w-full max-w-6xl flex-1 px-5 py-8 sm:px-8 sm:py-10">
      <RequireAdmin>
        <Dashboard />
      </RequireAdmin>
    </main>
  )
}
