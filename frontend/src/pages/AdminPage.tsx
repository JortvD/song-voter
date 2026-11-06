import { useCallback, useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { AdminDangerZone } from '../components/AdminDangerZone'
import { AdminInvites } from '../components/AdminInvites'
import { AdminPlaylist } from '../components/AdminPlaylist'
import { AdminRankingTable } from '../components/AdminRankingTable'
import { AdminReveal } from '../components/AdminReveal'
import { AdminSettings } from '../components/admin/AdminSettings'
import { AdminUsers } from '../components/AdminUsers'
import { RequireAdmin } from '../components/RequireAdmin'
import { StatusCard } from '../components/StatusCard'
import { AdminNav } from '../components/admin/AdminNav'
import type { AdminSection } from '../components/admin/AdminNav'
import { AdminOverview } from '../components/admin/AdminOverview'
import { AdminSectionHeader } from '../components/admin/AdminSectionHeader'
import { AdminStatusStrip } from '../components/admin/AdminStatusStrip'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { AdminInsights } from '../lib/types'

function initialSection(): AdminSection {
  if (typeof window === 'undefined') return 'overview'

  const params = new URLSearchParams(window.location.search)
  return params.has('spotify') ? 'playlist' : 'overview'
}

function MissionControl() {
  const { refresh: refreshPlaylist } = usePlaylist()
  const [section, setSection] = useState<AdminSection>(initialSection)
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
    void refreshPlaylist()
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
                description="Every account, with picks, duels and last activity. Expand a voter to rename them, reset a password, revoke sessions, clear picks or delete the account. You can also sort, search, or export the whole room."
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
                description="Set a Spotify playlist and its scheduled start, force-start it, re-sync its metadata, and watch preview and metadata coverage."
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
                description="Schedule or open The Reveal, pause and resume voting, and seal the shared standings again at any time."
              >
                <Link to="/standings" className="pv-admin-button">
                  Public standings
                </Link>
              </AdminSectionHeader>
              <AdminReveal
                key={`reveal-${refreshKey}`}
                onOpenPlaylist={() => setSection('playlist')}
              />
            </div>
          )}

          {section === 'invites' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The door"
                title="Invites"
                description="Generate invite codes, copy a code or a ready-to-send link, and revoke any code that has not been used yet."
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

          {section === 'settings' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="League rules"
                title="Settings"
                description="App-wide rules that apply to every voter. Changes take effect immediately."
              />
              <AdminSettings key={`settings-${refreshKey}`} />
            </div>
          )}

          {section === 'danger' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="Careful"
                title="Danger zone"
                description="Destructive actions for the active playlist. Reset picks and Remove playlist keep voters and invites; Full reset also clears them. All keep the Spotify connection."
              />
              <AdminDangerZone
                key={`danger-${refreshKey}`}
                onReset={refresh}
                onRemove={refresh}
                onOpenPlaylist={() => setSection('playlist')}
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
