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
import { AdminNav } from '../components/admin/AdminNav'
import type { AdminSection } from '../components/admin/AdminNav'
import { AdminOverview } from '../components/admin/AdminOverview'
import { AdminSectionHeader } from '../components/admin/AdminSectionHeader'
import { AdminStatusStrip } from '../components/admin/AdminStatusStrip'
import { ApiError, api } from '../lib/api'
import type { AdminInsights } from '../lib/types'

function MissionControl() {
  const [section, setSection] = useState<AdminSection>('overview')
  const [insights, setInsights] = useState<AdminInsights | null>(null)
  const [days, setDays] = useState(14)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)

    try {
      setInsights(await api.getAdminInsights(days))
    } catch (caught) {
      setError(
        caught instanceof ApiError
          ? caught.message
          : 'Could not load mission control data',
      )
    } finally {
      setLoading(false)
    }
  }, [days])

  useEffect(() => {
    void load()
  }, [load])

  function refresh() {
    setRefreshKey((value) => value + 1)
    void load()
  }

  return (
    <div className="space-y-5">
      <AdminStatusStrip
        insights={insights}
        loading={loading}
        error={error}
        onRefresh={refresh}
      />

      <div className="grid gap-6 lg:grid-cols-[220px_minmax(0,1fr)]">
        <AdminNav section={section} onChange={setSection} insights={insights} />

        <div className="min-w-0">
          {section === 'overview' && (
            <AdminOverview
              key={`overview-${refreshKey}`}
              insights={insights}
              loading={loading}
              error={error}
              days={days}
              onDaysChange={setDays}
              onRefresh={refresh}
              onNavigate={setSection}
            />
          )}

          {section === 'voters' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The room"
                title="Voters"
                description="Every account, with picks, duels and last activity. Sort the list, search by username, or export the whole room."
              >
                <a href="/api/admin/users.csv" className="pv-admin-button">
                  Export voters CSV
                </a>
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminUsers key={`voters-${refreshKey}`} />
            </div>
          )}

          {section === 'standings' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The official list"
                title="Standings"
                description="Every track with score, duels and wins, even while The Standings are sealed. Search, page through, or export the official CSV."
              >
                <a href="/api/admin/chart.csv" className="pv-admin-button">
                  Export standings CSV
                </a>
                <Link to="/standings" className="pv-admin-button">
                  Public standings
                </Link>
              </AdminSectionHeader>
              <AdminRankingTable key={`standings-${refreshKey}`} />
            </div>
          )}

          {section === 'playlist' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The tape"
                title="Playlist"
                description="Import or re-sync the Spotify playlist, watch preview coverage and keep the current session healthy."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminPlaylist key={`playlist-${refreshKey}`} />
            </div>
          )}

          {section === 'reveal' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The switch"
                title="The Reveal"
                description="Open or seal the shared standings. Picking stays open either way, and you can hide the list again at any time."
              >
                <Link to="/standings" className="pv-admin-button">
                  Public standings
                </Link>
              </AdminSectionHeader>
              <AdminReveal key={`reveal-${refreshKey}`} />
            </div>
          )}

          {section === 'invites' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The door"
                title="Invites"
                description="Generate invite codes, copy them for voters, and revoke any code that has not been used yet."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminInvites key={`invites-${refreshKey}`} />
            </div>
          )}

          {section === 'runtime' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The engine room"
                title="Runtime"
                description="Backend version, environment, uptime and PostgreSQL readiness. This card refreshes itself every ten seconds."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh dashboard
                </button>
              </AdminSectionHeader>
              <StatusCard />
            </div>
          )}

          {section === 'danger' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="Careful"
                title="Danger zone"
                description="Destructive actions for the active playlist. Both require a typed confirmation, and both keep voters, invites and the Spotify connection."
              />
              <AdminDangerZone
                key={`danger-${refreshKey}`}
                onReset={refresh}
                onRemove={refresh}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export function AdminPage() {
  return (
    <main className="flex-1 bg-paper text-ink">
      <div className="mx-auto w-full max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
        <RequireAdmin>
          <MissionControl />
        </RequireAdmin>
      </div>
    </main>
  )
}
