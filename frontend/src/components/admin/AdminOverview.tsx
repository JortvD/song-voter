import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'

import { AdminSectionHeader } from './AdminSectionHeader'
import type { AdminSection } from './AdminNav'
import { AdminProgressPanel } from './AdminProgressPanel'
import { AdminSongInsightsPanel } from './AdminSongInsightsPanel'
import { AdminVoteQualityPanel } from './AdminVoteQualityPanel'
import { StoryCardGrid, StoryLeaderboards } from '../StoryCards'
import { api } from '../../lib/api'
import type { AdminInsights, ChartStories } from '../../lib/types'

type Props = {
  insights: AdminInsights | null
  loading: boolean
  error: string | null
  days: number
  onDaysChange: (days: number) => void
  onRefresh: () => void
  onNavigate: (section: AdminSection) => void
  onOpenSong: (id: string) => void
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatRangeDate(value: string) {
  return new Date(`${value}T00:00:00`).toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
  })
}

function formatWhen(value: string) {
  return new Date(value).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

function MetricCard({
  label,
  value,
  hint,
  sub,
}: {
  label: string
  value: string
  hint?: string
  sub?: string
}) {
  return (
    <div className="pv-admin-stat">
      <p className="pv-admin-label">{label}</p>
      <p className="mt-2 font-serif text-3xl leading-none text-ink">{value}</p>
      {hint && (
        <p className="mt-1 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {hint}
        </p>
      )}
      {sub && (
        <p className="mt-2 border-t border-line pt-2 text-xs leading-relaxed text-muted">
          {sub}
        </p>
      )}
    </div>
  )
}

function Meter({
  label,
  value,
  percentage,
}: {
  label: string
  value: string
  percentage: number
}) {
  return (
    <div>
      <div className="flex items-center justify-between gap-3 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
        <span>{label}</span>
        <span className="text-ink">{value}</span>
      </div>
      <div className="pv-admin-meter mt-1.5">
        <div
          className="pv-admin-meter-fill"
          style={{ width: `${Math.min(100, Math.max(0, percentage))}%` }}
        />
      </div>
    </div>
  )
}

function TrackCover({ src, alt }: { src: string | null; alt: string }) {
  if (src) {
    return (
      <img
        src={src}
        alt={alt}
        className="size-9 shrink-0 rounded-lg border border-line object-cover"
      />
    )
  }

  return (
    <span className="grid size-9 shrink-0 place-items-center rounded-lg border border-line bg-paper font-serif text-sm text-muted">
      ♪
    </span>
  )
}

function DailyChart({ data }: { data: AdminInsights['daily'] }) {
  const [selected, setSelected] = useState<AdminInsights['daily'][number] | null>(
    null,
  )
  const max = Math.max(1, ...data.map((day) => day.votes))
  const total = data.reduce((sum, day) => sum + day.votes, 0)
  const labelEvery = data.length > 30 ? 14 : data.length > 14 ? 3 : 2

  useEffect(() => {
    setSelected(null)
  }, [data])

  return (
    <div className="pv-admin-panel">
      <div className="flex items-end justify-between gap-3">
        <div>
          <p className="pv-admin-label">
            Activity · {data.length === 1 ? '1 day' : `${data.length} days`}
          </p>
          <h3 className="font-serif text-xl text-ink">Picks per day</h3>
        </div>
        <span className="pv-admin-label">{formatNumber(total)} total</span>
      </div>

      <div
        className={`mt-5 flex h-36 items-end ${data.length > 30 ? 'gap-px' : 'gap-1'}`}
      >
        {data.map((day) => (
          <button
            key={day.date}
            type="button"
            title={`${day.date}: ${day.votes} picks, ${day.voters} voters`}
            aria-label={`${day.date}: ${day.votes} picks, ${day.voters} voters`}
            onClick={() => setSelected(day)}
            className="group flex h-full min-w-0 flex-1 flex-col justify-end border-0 bg-transparent p-0"
          >
            <div
              className={`w-full rounded-t-sm transition-colors ${
                selected?.date === day.date
                  ? 'bg-[var(--color-butter)] ring-2 ring-ink ring-offset-1'
                  : 'bg-[var(--color-cobalt)] group-hover:bg-[var(--color-butter)]'
              }`}
              style={{
                height: day.votes === 0 ? '2px' : `${Math.max(4, (day.votes / max) * 100)}%`,
                opacity: day.votes === 0 ? 0.25 : 1,
              }}
            />
          </button>
        ))}
      </div>

      <div className="mt-2 flex gap-1 font-mono text-[8px] uppercase tracking-[0.08em] text-muted">
        {data.map((day, index) => (
          <span key={day.date} className="min-w-0 flex-1 text-center">
            {index % labelEvery === 0 ? day.date.slice(5) : ''}
          </span>
        ))}
      </div>

      <div className="mt-3 flex flex-wrap items-center justify-between gap-2 border-t border-line pt-2">
        {selected ? (
          <span className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
            <span className="text-ink">{selected.date}</span> ·{' '}
            {formatNumber(selected.votes)} picks · {formatNumber(selected.voters)}{' '}
            voters
          </span>
        ) : (
          <span className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
            Click a bar to inspect a day
          </span>
        )}
        {selected && (
          <button
            type="button"
            onClick={() => setSelected(null)}
            className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted underline decoration-line underline-offset-4 hover:text-ink"
          >
            Clear
          </button>
        )}
      </div>
    </div>
  )
}

function DistributionPanel({ data }: { data: AdminInsights['pick_distribution'] }) {
  const max = Math.max(1, ...data.map((bucket) => bucket.voters))

  return (
    <div className="pv-admin-panel">
      <p className="pv-admin-label">Voter distribution · all-time</p>
      <h3 className="font-serif text-xl text-ink">How deep is the room?</h3>

      <div className="mt-4 space-y-2.5">
        {data.length === 0 ? (
          <p className="text-sm text-muted">No picks yet.</p>
        ) : (
          data.map((bucket) => (
            <div key={bucket.label}>
              <div className="flex items-center justify-between gap-3 font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                <span>{bucket.label}</span>
                <span className="text-ink">{formatNumber(bucket.voters)}</span>
              </div>
              <div className="pv-admin-meter mt-1">
                <div
                  className="pv-admin-meter-fill"
                  style={{ width: `${(bucket.voters / max) * 100}%` }}
                />
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  )
}

function CoveragePanel({ insights }: { insights: AdminInsights }) {
  const previewPct =
    insights.total_tracks > 0
      ? ((insights.total_tracks - insights.previews_missing) /
          insights.total_tracks) *
        100
      : 0
  const invitePct =
    insights.invites_total > 0
      ? (insights.invites_used / insights.invites_total) * 100
      : 0

  return (
    <div className="pv-admin-panel">
      <p className="pv-admin-label">Coverage · all-time</p>
      <h3 className="font-serif text-xl text-ink">What is left to hear?</h3>

      <div className="mt-4 space-y-4">
        <Meter
          label="Songs compared"
          value={`${formatNumber(insights.tracks_compared)} / ${formatNumber(insights.total_tracks)}`}
          percentage={insights.coverage_pct}
        />
        <Meter
          label="Previews available"
          value={`${formatNumber(insights.total_tracks - insights.previews_missing)} / ${formatNumber(insights.total_tracks)}`}
          percentage={previewPct}
        />
        <Meter
          label="Invites used"
          value={`${formatNumber(insights.invites_used)} / ${formatNumber(insights.invites_total)}`}
          percentage={invitePct}
        />
      </div>

      <div className="mt-4 grid grid-cols-2 gap-2 border-t border-line pt-4">
        <div>
          <p className="pv-admin-label text-[8px]">Uncompared</p>
          <p className="mt-1 font-serif text-xl text-ink">
            {formatNumber(insights.tracks_uncompared)}
          </p>
        </div>
        <div>
          <p className="pv-admin-label text-[8px]">Preview gaps</p>
          <p className="mt-1 font-serif text-xl text-ink">
            {formatNumber(insights.previews_missing)}
          </p>
        </div>
      </div>
    </div>
  )
}

function InvitePanel({
  insights,
  onNavigate,
}: {
  insights: AdminInsights
  onNavigate: (section: AdminSection) => void
}) {
  return (
    <div className="pv-admin-panel">
      <p className="pv-admin-label">Invite pipeline</p>
      <h3 className="font-serif text-xl text-ink">The door</h3>

      <div className="mt-4 grid grid-cols-2 gap-2">
        {[
          { label: 'Total', value: insights.invites_total },
          { label: 'Open', value: insights.invites_open },
          { label: 'Used', value: insights.invites_used },
          { label: 'Expired', value: insights.invites_expired },
        ].map((item) => (
          <div key={item.label} className="rounded-xl border border-line bg-paper p-3">
            <p className="pv-admin-label text-[8px]">{item.label}</p>
            <p className="mt-1 font-serif text-xl text-ink">
              {formatNumber(item.value)}
            </p>
          </div>
        ))}
      </div>

      <button
        type="button"
        onClick={() => onNavigate('invites')}
        className="pv-admin-button mt-4 w-full"
      >
        Manage invites
      </button>
    </div>
  )
}

function TopTracksPanel({
  data,
  onNavigate,
}: {
  data: AdminInsights['top_tracks']
  onNavigate: (section: AdminSection) => void
}) {
  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">Official standings · all-time</p>
          <h3 className="font-serif text-xl text-ink">Top tracks</h3>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('standings')}
          className="pv-admin-button"
        >
          All standings
        </button>
      </div>

      {data.length === 0 ? (
        <p className="mt-4 text-sm text-muted">No tracks yet.</p>
      ) : (
        <div className="mt-3 overflow-x-auto">
          <table className="pv-admin-table w-full min-w-[420px] table-fixed">
            <thead>
              <tr>
                <th className="w-8">#</th>
                <th>Song</th>
                <th className="w-14 text-right">Score</th>
                <th className="w-14 text-right">Duels</th>
                <th className="w-14 text-right">Wins</th>
              </tr>
            </thead>
            <tbody>
              {data.map((track, index) => (
                <tr key={track.id}>
                  <td className="font-serif text-lg text-muted">
                    {String(index + 1).padStart(2, '0')}
                  </td>
                  <td>
                    <span className="flex items-center gap-2.5">
                      <TrackCover src={track.album_image_url} alt="" />
                      <span className="min-w-0">
                        <span className="block truncate font-medium text-ink">
                          {track.title}
                        </span>
                        <span className="block truncate font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
                          {track.artists}
                        </span>
                      </span>
                    </span>
                  </td>
                  <td className="text-right font-mono text-xs text-ink">
                    {Math.round(track.rating)}
                  </td>
                  <td className="text-right font-mono text-xs text-muted">
                    {formatNumber(track.comparisons)}
                  </td>
                  <td className="text-right font-mono text-xs text-muted">
                    {formatNumber(track.wins)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

function TopVotersPanel({
  data,
  onNavigate,
}: {
  data: AdminInsights['top_voters']
  onNavigate: (section: AdminSection) => void
}) {
  return (
    <div className="pv-admin-panel">
      <div className="flex flex-wrap items-end justify-between gap-2">
        <div>
          <p className="pv-admin-label">The room · all-time</p>
          <h3 className="font-serif text-xl text-ink">Top voters</h3>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('voters')}
          className="pv-admin-button"
        >
          All voters
        </button>
      </div>

      {data.length === 0 ? (
        <p className="mt-4 text-sm text-muted">No voters yet.</p>
      ) : (
        <div className="mt-3 overflow-x-auto">
          <table className="pv-admin-table w-full min-w-[360px] table-fixed">
            <thead>
              <tr>
                <th className="w-8">#</th>
                <th>Voter</th>
                <th className="w-14 text-right">Picks</th>
                <th className="w-14 text-right">Duels</th>
                <th className="w-24 text-right">Last active</th>
              </tr>
            </thead>
            <tbody>
              {data.map((voter, index) => (
                <tr key={voter.user_id}>
                  <td className="font-serif text-lg text-muted">
                    {String(index + 1).padStart(2, '0')}
                  </td>
                  <td className="truncate font-medium text-ink">{voter.username}</td>
                  <td className="text-right font-mono text-xs text-ink">
                    {formatNumber(voter.picks)}
                  </td>
                  <td className="text-right font-mono text-xs text-muted">
                    {formatNumber(voter.duels)}
                  </td>
                  <td className="whitespace-nowrap text-right font-mono text-[8px] uppercase tracking-[0.08em] text-muted">
                    {formatWhen(voter.last_active_at)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

function StoriesPanel({ stories }: { stories: ChartStories | null }) {
  const hasStories = Boolean(
    stories &&
      (stories.most_duelled ||
        stories.top_voter ||
        stories.cards.length > 0 ||
        stories.leaderboards.length > 0),
  )

  return (
    <div className="pv-admin-panel">
      <p className="pv-admin-label">Liner notes</p>
      <h3 className="font-serif text-xl text-ink">The stories</h3>

      {!hasStories || !stories ? (
        <p className="mt-4 text-sm text-muted">
          The stories will appear once there are enough picks.
        </p>
      ) : (
        <>
          <div className="mt-3 space-y-3">
            {stories.most_duelled && (
              <div className="rounded-xl border border-line bg-paper p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="pv-admin-chip">Most duelled</span>
                  <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
                    {formatNumber(stories.most_duelled.comparisons)} duels
                  </span>
                </div>
                <div className="mt-2 flex items-center gap-2.5">
                  <TrackCover
                    src={stories.most_duelled.track.album_image_url}
                    alt=""
                  />
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-ink">
                      {stories.most_duelled.track.title}
                    </p>
                    <p className="truncate font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                      {stories.most_duelled.track.artists}
                    </p>
                  </div>
                </div>
              </div>
            )}

            {stories.top_voter && (
              <div className="rounded-xl border border-line bg-paper p-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="pv-admin-chip">Top voter</span>
                  <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
                    {formatNumber(stories.top_voter.picks)} picks
                  </span>
                </div>
                <div className="mt-2 flex items-center gap-2.5">
                  <span className="grid size-9 place-items-center rounded-full border border-line bg-ink/10 font-serif text-lg text-ink">
                    {stories.top_voter.username.slice(0, 1).toUpperCase()}
                  </span>
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-ink">
                      {stories.top_voter.username}
                    </p>
                    <p className="font-mono text-[9px] uppercase tracking-[0.1em] text-muted">
                      Most active listener
                    </p>
                  </div>
                </div>
              </div>
            )}
          </div>

          {stories.cards.length > 0 && (
            <div className="mt-4">
              <div className="flex items-end justify-between gap-2 border-b border-line pb-1">
                <p className="pv-admin-label">Superlatives</p>
                <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
                  {stories.cards.length} cards
                </span>
              </div>
              <StoryCardGrid cards={stories.cards} />
            </div>
          )}

          {stories.leaderboards.length > 0 && (
            <div className="mt-4">
              <div className="flex items-end justify-between gap-2 border-b border-line pb-1">
                <p className="pv-admin-label">Top 5 leaderboards</p>
                <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
                  {stories.leaderboards.length} lists
                </span>
              </div>
              <StoryLeaderboards leaderboards={stories.leaderboards} />
            </div>
          )}
        </>
      )}
    </div>
  )
}

export function AdminOverview({
  insights,
  loading,
  error,
  days,
  onDaysChange,
  onRefresh,
  onNavigate,
  onOpenSong,
}: Props) {
  const [stories, setStories] = useState<ChartStories | null>(null)

  useEffect(() => {
    if (!insights) return

    let active = true
    api
      .getAdminStories()
      .then((data) => {
        if (active) setStories(data)
      })
      .catch(() => {
        if (active) setStories(null)
      })

    return () => {
      active = false
    }
  }, [insights?.total_picks, insights?.total_duels])

  if (!insights) {
    return (
      <div className="space-y-4">
        <AdminSectionHeader
          eyebrow="Mission control"
          title="Overview"
          description="A live read on the tape, the room and the picks."
        />
        {error ? (
          <div className="pv-admin-panel">
            <p className="text-sm text-danger">{error}</p>
            <button
              type="button"
              onClick={onRefresh}
              className="pv-admin-button mt-3"
            >
              Try again
            </button>
          </div>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            {[0, 1, 2, 3, 4, 5, 6, 7].map((index) => (
              <div
                key={index}
                className="h-32 animate-pulse rounded-2xl border border-line bg-card"
              />
            ))}
          </div>
        )}
      </div>
    )
  }

  const previewPct =
    insights.total_tracks > 0
      ? ((insights.total_tracks - insights.previews_missing) /
          insights.total_tracks) *
        100
      : 0

  return (
    <div className="space-y-6">
      <AdminSectionHeader
        eyebrow="Mission control"
        title="Overview"
        description="A live read on the tape, the room and the picks. Use the range buttons to look back through the last 90 days."
      >
        <button type="button" onClick={onRefresh} className="pv-admin-button">
          {loading ? 'Refreshing…' : 'Refresh'}
        </button>
        <a href="/api/admin/chart.csv" className="pv-admin-button">
          Standings CSV
        </a>
        <a href="/api/admin/users.csv" className="pv-admin-button">
          Voters CSV
        </a>
        <Link to="/standings" className="pv-admin-button">
          Public standings
        </Link>
      </AdminSectionHeader>

      <div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border-2 border-line bg-card px-3 py-2">
        <div className="flex flex-wrap items-center gap-1">
          <span className="pv-admin-label mr-1">Look back</span>
          {[1, 7, 14, 30, 90].map((value) => (
            <button
              key={value}
              type="button"
              onClick={() => onDaysChange(value)}
              aria-pressed={days === value}
              className={`rounded-full border-2 px-3 py-1.5 font-mono text-[9px] font-semibold uppercase tracking-[0.12em] transition ${
                days === value
                  ? 'border-ink bg-ink text-paper'
                  : 'border-line bg-card text-muted hover:border-ink hover:text-ink'
              }`}
            >
              {value === 1 ? 'Today' : `${value}d`}
            </button>
          ))}
        </div>
        <span className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
          {formatRangeDate(insights.range_start)} → {formatRangeDate(insights.range_end)}
        </span>
      </div>

      {error && (
        <p className="rounded-xl border border-danger bg-danger/5 px-3 py-2 text-sm text-danger">
          {error}
        </p>
      )}

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="Picks in range"
          value={formatNumber(insights.range_picks)}
          hint={`${formatNumber(insights.picks_today)} today · ${formatNumber(insights.picks_7d)} in 7 days`}
          sub={`${insights.range_picks_per_voter.toFixed(1)} per voter · ${formatRangeDate(insights.range_start)} → ${formatRangeDate(insights.range_end)}`}
        />
        <MetricCard
          label="Voters in range"
          value={formatNumber(insights.range_voters)}
          hint={`${formatNumber(insights.active_voters_24h)} active 24h · ${formatNumber(insights.active_voters_7d)} active 7d`}
          sub={`${formatNumber(insights.total_users)} registered · ${formatNumber(insights.admins)} admins`}
        />
        <MetricCard
          label="Duels in range"
          value={formatNumber(insights.range_duels)}
          hint={`${insights.range_duels_per_voter.toFixed(1)} per voter`}
          sub={`${formatNumber(insights.total_duels)} all-time duels · ${insights.coverage_pct.toFixed(0)}% coverage`}
        />
        <MetricCard
          label="Songs"
          value={formatNumber(insights.total_tracks)}
          hint={`${formatNumber(insights.tracks_compared)} compared`}
          sub={`${formatNumber(insights.tracks_uncompared)} still unheard`}
        />
        <MetricCard
          label="Previews"
          value={`${formatNumber(insights.total_tracks - insights.previews_missing)} / ${formatNumber(insights.total_tracks)}`}
          hint={`${formatNumber(insights.previews_missing)} missing`}
          sub={`${Math.round(previewPct)}% preview coverage`}
        />
        <MetricCard
          label="Invites"
          value={formatNumber(insights.invites_open)}
          hint={`${formatNumber(insights.invites_used)} used · ${formatNumber(insights.invites_expired)} expired`}
          sub={`${formatNumber(insights.invites_total)} generated in total`}
        />
        <MetricCard
          label="Active 24h"
          value={formatNumber(insights.active_voters_24h)}
          hint="Voters active in the last 24h"
          sub={`${formatNumber(insights.picks_today)} picks landed today`}
        />
        <MetricCard
          label="Coverage"
          value={`${insights.coverage_pct.toFixed(0)}%`}
          hint={`${formatNumber(insights.tracks_compared)} / ${formatNumber(insights.total_tracks)} songs`}
          sub={`${insights.picks_per_voter.toFixed(1)} all-time picks per voter`}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <DailyChart data={insights.daily} />
        <AdminVoteQualityPanel />
      </div>

      <AdminProgressPanel />

      <div className="grid gap-4 xl:grid-cols-3">
        <DistributionPanel data={insights.pick_distribution} />
        <CoveragePanel insights={insights} />
        <InvitePanel insights={insights} onNavigate={onNavigate} />
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        <TopTracksPanel data={insights.top_tracks} onNavigate={onNavigate} />
        <TopVotersPanel data={insights.top_voters} onNavigate={onNavigate} />
      </div>

      <AdminSongInsightsPanel onOpenSong={onOpenSong} />

      <div className="pv-admin-panel flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="pv-admin-label">Live feed moved</p>
          <h3 className="font-serif text-xl text-ink">Every duel, including no votes</h3>
          <p className="mt-1 max-w-xl text-sm leading-relaxed text-muted">
            The old recent-picks list has been replaced by the Duels section:
            filters, upsets, decision times and full row details.
          </p>
        </div>
        <button
          type="button"
          onClick={() => onNavigate('duels')}
          className="pv-admin-button pv-admin-button-accent"
        >
          Open the duel feed
        </button>
      </div>

      <StoriesPanel stories={stories} />
    </div>
  )
}
