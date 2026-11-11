import { useCallback, useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { AdminDangerZone } from '../components/AdminDangerZone'
import { AdminInvites } from '../components/AdminInvites'
import { AdminPlaylist } from '../components/AdminPlaylist'
import { AdminRankingTable } from '../components/AdminRankingTable'
import { AdminReveal } from '../components/AdminReveal'
import { AdminSettings } from '../components/admin/AdminSettings'
import { AdminVoters } from '../components/admin/AdminVoters'
import { RequireAdmin } from '../components/RequireAdmin'
import { StatusCard } from '../components/StatusCard'
import { AdminNav } from '../components/admin/AdminNav'
import type { AdminSection } from '../components/admin/AdminNav'
import { AdminDuelFeed, BiggestUpsets } from '../components/admin/AdminDuels'
import { AdminOverview } from '../components/admin/AdminOverview'
import { AdminPairing } from '../components/admin/AdminPairing'
import { AdminSectionHeader } from '../components/admin/AdminSectionHeader'
import { SongDeepDive } from '../components/admin/AdminSongDeepDive'
import { AdminStatusStrip } from '../components/admin/AdminStatusStrip'
import { usePlaylist } from '../context/PlaylistContext'
import { ApiError, api } from '../lib/api'
import type { AdminInsights } from '../lib/types'

const SECTIONS: AdminSection[] = [
  'overview',
  'duels',
  'voters',
  'standings',
  'pairing',
  'playlist',
  'reveal',
  'invites',
  'settings',
  'runtime',
  'danger',
]

function initialSection(): AdminSection {
  if (typeof window === 'undefined') return 'overview'

  const params = new URLSearchParams(window.location.search)
  if (params.has('spotify')) return 'playlist'
  const requested = params.get('section')
  return SECTIONS.includes(requested as AdminSection)
    ? (requested as AdminSection)
    : 'overview'
}

function MissionControl() {
  const { refresh: refreshPlaylist } = usePlaylist()
  const [section, setSection] = useState<AdminSection>(initialSection)
  const [insights, setInsights] = useState<AdminInsights | null>(null)
  const [days, setDays] = useState(14)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [refreshKey, setRefreshKey] = useState(0)
  const [songId, setSongId] = useState<string | null>(null)

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
              onOpenSong={setSongId}
            />
          )}

          {section === 'duels' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="Every shown duel"
                title="Duels"
                description="The newest duels first, including no-vote rows. Filter by voter, song, result or time, then expand a row for ratings, win chances, K, comparison counts and the vote sequence. A no-vote row can be a skip, an undone vote or the duel on screen right now."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminDuelFeed key={`duels-${refreshKey}`} onOpenSong={setSongId} />
              <BiggestUpsets onOpenSong={setSongId} />
            </div>
          )}

          {section === 'voters' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The room"
                title="Voters"
                description="Every account with decision-time, agreement and session metrics. Expand a voter to manage the account; click their name for the full detail page. Flags are informational only and never shown to voters."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminVoters key={`voters-${refreshKey}`} />
            </div>
          )}

          {section === 'standings' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The official list"
                title="Standings"
                description="Every track with score, duels and wins, even while The Standings are sealed. Use more columns for 24h rank change, win rate, form and no-vote rate. Click a song for its deep dive."
              >
                <a href="/api/admin/chart.csv" className="pv-admin-button">
                  Export standings CSV
                </a>
                <Link to="/standings" className="pv-admin-button">
                  Public standings
                </Link>
              </AdminSectionHeader>
              <AdminRankingTable
                key={`standings-${refreshKey}`}
                onOpenSong={setSongId}
              />
            </div>
          )}

          {section === 'pairing' && (
            <div className="space-y-4">
              <AdminSectionHeader
                eyebrow="The algorithm"
                title="Pairing health"
                description="Whether rank-weighted selection is active, how close it is to settling, and the quality of the pairs it serves. Live pairing settings are in Settings."
              >
                <button type="button" onClick={refresh} className="pv-admin-button">
                  Refresh
                </button>
              </AdminSectionHeader>
              <AdminPairing key={`pairing-${refreshKey}`} />
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
                onReset={refresh}
                onRemove={refresh}
                onOpenPlaylist={() => setSection('playlist')}
              />
            </div>
          )}
        </div>
      </div>

      {songId && (
        <SongDeepDive trackId={songId} onClose={() => setSongId(null)} />
      )}
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
