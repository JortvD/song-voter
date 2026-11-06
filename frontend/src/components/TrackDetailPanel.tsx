import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'

import { ApiError, api } from '../lib/api'
import type {
  OfficialTrackStats,
  PersonalTrackStats,
  TrackDetail,
  TrackDuel,
} from '../lib/types'

type DetailScope = 'official' | 'personal'

type TrackDetailPanelProps = {
  trackId: string
  preferredScope: DetailScope
}

type Metric = {
  label: string
  value: string
}

function formatNumber(value: number) {
  return value.toLocaleString()
}

function formatPercent(value: number | null) {
  return value === null ? '—' : `${Math.round(value * 100)}%`
}

function formatScore(value: number | null) {
  return value === null ? '—' : Math.round(value).toString()
}

function formatDate(value: string | null) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  })
}

function formatShortDate(value: string) {
  if (!value) return ''
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''
  return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}

function formatDuration(value: number | null) {
  if (!value) return ''
  const totalSeconds = Math.round(value / 1000)
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, '0')}`
}

function topPercent(rank: number | null, total: number) {
  if (rank === null || total <= 0) return null
  return Math.max(1, Math.ceil((rank / total) * 100))
}

function MetricList({ metrics }: { metrics: Metric[] }) {
  return (
    <span className="flex flex-wrap items-baseline gap-x-2 gap-y-0">
      {metrics.map((metric) => (
        <span
          key={metric.label}
          className="inline-flex items-baseline gap-1 whitespace-nowrap"
        >
          <span className="font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
            {metric.label}
          </span>
          <span className="font-serif text-xs leading-none">{metric.value}</span>
        </span>
      ))}
    </span>
  )
}

function ContextRow({
  label,
  children,
}: {
  label: string
  children: ReactNode
}) {
  return (
    <div className="flex min-w-0 items-baseline gap-1.5">
      <span className="shrink-0 font-mono text-[7px] font-semibold uppercase tracking-[0.1em] text-muted">
        {label}
      </span>
      <span className="min-w-0 break-words text-[10px] leading-snug">
        {children}
      </span>
    </div>
  )
}

function RecordLine({
  kind,
  stats,
}: {
  kind: 'official' | 'personal'
  stats: OfficialTrackStats | PersonalTrackStats
}) {
  const official = kind === 'official'
  const officialStats = official ? (stats as OfficialTrackStats) : null
  const personalStats = official ? null : (stats as PersonalTrackStats)

  if (personalStats && personalStats.comparisons === 0) {
    return (
      <div className="flex min-w-0 items-baseline gap-2 rounded-md border border-line bg-card px-2 py-1.5">
        <span className="font-mono text-[7px] font-semibold uppercase tracking-[0.1em] text-muted">
          You
        </span>
        <span className="text-[10px] text-muted">
          You have not heard this one yet.
        </span>
      </div>
    )
  }

  const metrics: Metric[] = [
    { label: 'score', value: formatScore(stats.rating) },
  ]

  if (stats.comparisons > 0) {
    metrics.push(
      { label: 'wins', value: formatNumber(stats.wins) },
      { label: 'losses', value: formatNumber(stats.losses) },
      { label: 'duels', value: formatNumber(stats.comparisons) },
    )
    if (stats.win_rate !== null) {
      metrics.push({ label: 'win', value: formatPercent(stats.win_rate) })
    }
  }

  if (officialStats && officialStats.voters > 0) {
    metrics.push({ label: 'voters', value: formatNumber(officialStats.voters) })
  }

  const meta: string[] = []

  if (officialStats) {
    if (officialStats.unique_opponents > 0) {
      meta.push(`${formatNumber(officialStats.unique_opponents)} rivals`)
    }
    if (
      officialStats.total_rating_average !== null &&
      officialStats.comparisons > 0
    ) {
      const delta = Math.round(
        officialStats.rating - officialStats.total_rating_average,
      )
      meta.push(`${delta >= 0 ? '+' : ''}${delta} vs tape avg`)
    }
    if (officialStats.first_duel_at) {
      meta.push(`first ${formatDate(officialStats.first_duel_at)}`)
    }
    if (
      officialStats.last_duel_at &&
      officialStats.last_duel_at !== officialStats.first_duel_at
    ) {
      meta.push(`last ${formatDate(officialStats.last_duel_at)}`)
    }
  } else if (personalStats?.first_duel_at) {
    meta.push(`first ${formatDate(personalStats.first_duel_at)}`)
    if (
      personalStats.last_duel_at &&
      personalStats.last_duel_at !== personalStats.first_duel_at
    ) {
      meta.push(`last ${formatDate(personalStats.last_duel_at)}`)
    }
  }

  const rank = official
    ? officialStats
      ? `#${formatNumber(officialStats.rank)}/${formatNumber(officialStats.total_tracks)}`
      : null
    : personalStats?.rank === null || personalStats?.rank === undefined
      ? 'not ranked'
      : `#${formatNumber(personalStats.rank)}/${formatNumber(personalStats.total_ranked)}`

  const percentile = official
    ? officialStats
      ? topPercent(officialStats.rank, officialStats.total_tracks)
      : null
    : personalStats
      ? topPercent(personalStats.rank, personalStats.total_ranked)
      : null

  return (
    <div className="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-0.5 rounded-md border border-line bg-card px-2 py-1.5">
      <span className="font-mono text-[7px] font-semibold uppercase tracking-[0.1em] text-muted">
        {official ? 'Group' : 'You'}
      </span>
      <span className="font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
        {rank}
        {percentile !== null ? ` · top ${percentile}%` : ''}
      </span>
      <MetricList metrics={metrics} />
      {meta.length > 0 && (
        <span className="w-full font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
          {meta.join(' · ')}
        </span>
      )}
    </div>
  )
}

function DuelLine({ duel }: { duel: TrackDuel }) {
  return (
    <div className="flex min-w-0 items-baseline gap-1.5 border-b border-line py-1 last:border-b-0">
      <span
        className={`shrink-0 rounded border border-ink px-1 py-0.5 font-mono text-[7px] font-semibold uppercase tracking-[0.08em] ${
          duel.won
            ? 'bg-[var(--playlist-accent)] text-[var(--playlist-accent-ink)]'
            : 'bg-paper text-muted'
        }`}
      >
        {duel.won ? 'W' : 'L'}
      </span>
      <span className="min-w-0 flex-1 truncate text-[10px] leading-tight">
        {duel.opponent.title}
        <span className="ml-1 font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
          {duel.opponent.artists}
        </span>
      </span>
      <span className="shrink-0 font-mono text-[7px] uppercase tracking-[0.06em] text-muted">
        {formatShortDate(duel.occurred_at)}
      </span>
    </div>
  )
}

export function TrackDetailPanel({
  trackId,
  preferredScope,
}: TrackDetailPanelProps) {
  const [detail, setDetail] = useState<TrackDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    const controller = new AbortController()
    setLoading(true)
    setError(null)

    api
      .getTrackDetail(trackId, controller.signal)
      .then((data) => {
        if (active) setDetail(data)
      })
      .catch((caught: unknown) => {
        if (!active || controller.signal.aborted) return
        setError(
          caught instanceof ApiError
            ? caught.message
            : 'Could not load this song’s stats',
        )
      })
      .finally(() => {
        if (active) setLoading(false)
      })

    return () => {
      active = false
      controller.abort()
    }
  }, [trackId])

  if (loading) {
    return (
      <div className="border-t border-line bg-paper/70 px-2.5 py-2">
        <div className="h-12 animate-pulse rounded-md border border-line bg-card" />
      </div>
    )
  }

  if (error) {
    return (
      <div className="border-t border-line bg-paper/70 px-2.5 py-2">
        <p className="text-[10px] text-danger">{error}</p>
      </div>
    )
  }

  if (!detail) return null

  const track = detail.track
  const records: Array<
    { kind: 'official'; stats: OfficialTrackStats } | {
      kind: 'personal'
      stats: PersonalTrackStats
    }
  > = []

  for (const scope of preferredScope === 'personal'
    ? (['personal', 'official'] as const)
    : (['official', 'personal'] as const)) {
    if (scope === 'official' && detail.official) {
      records.push({ kind: 'official', stats: detail.official })
    }
    if (scope === 'personal' && detail.personal) {
      records.push({ kind: 'personal', stats: detail.personal })
    }
  }

  const albumMeta: string[] = []
  if (detail.album) {
    albumMeta.push(
      `${formatNumber(detail.album.playlist_track_count)} on tape`,
    )
    if (detail.album.average_rating !== null) {
      albumMeta.push(`avg ${formatScore(detail.album.average_rating)}`)
    }
    if (detail.album.release_year !== null) {
      albumMeta.push(String(detail.album.release_year))
    }
    if (detail.album.album_type) {
      albumMeta.push(detail.album.album_type)
    }
  }

  const noteParts: string[] = []
  if (track.release_year !== null) {
    noteParts.push(String(track.release_year))
  } else if (track.decade !== null) {
    noteParts.push(`${track.decade}s`)
  }
  if (track.primary_genre) noteParts.push(track.primary_genre)
  if (track.duration_ms !== null) {
    noteParts.push(formatDuration(track.duration_ms))
  }
  if (track.track_number !== null) {
    noteParts.push(
      track.disc_number !== null
        ? `disc ${track.disc_number}, track ${track.track_number}`
        : `track ${track.track_number}`,
    )
  }

  const primary = records[0]
  const recentDuels = (primary?.stats.recent_duels ?? []).slice(0, 4)

  return (
    <div className="border-t border-line bg-paper/70 px-2.5 py-2 sm:px-3">
      <div className="grid gap-1 lg:grid-cols-2">
        {records.map((record) => (
          <RecordLine
            key={record.kind}
            kind={record.kind}
            stats={record.stats}
          />
        ))}
      </div>

      <div className="mt-1 grid gap-x-3 gap-y-0.5">
        {detail.album && (
          <ContextRow label="Album">
            <span className="font-medium">{detail.album.name}</span>
            {albumMeta.length > 0 && (
              <span className="ml-1 font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
                {albumMeta.join(' · ')}
              </span>
            )}
            {detail.album.best_track && (
              <span className="ml-1 font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
                standout {detail.album.best_track.title}
              </span>
            )}
          </ContextRow>
        )}

        {detail.artists.length > 0 && (
          <ContextRow label="Artists">
            {detail.artists.map((artist, index) => (
              <span key={artist.id}>
                {index > 0 && ' · '}
                <span className="font-medium">{artist.name}</span>
                {artist.genre ? (
                  <span className="text-muted"> ({artist.genre})</span>
                ) : null}
                <span className="font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
                  {artist.track_count > 0
                    ? ` ${formatNumber(artist.track_count)} tracks`
                    : ''}
                  {artist.average_rating !== null
                    ? ` · avg ${formatScore(artist.average_rating)}`
                    : ''}
                </span>
              </span>
            ))}
          </ContextRow>
        )}

        {(noteParts.length > 0 || track.spotify_id) && (
          <ContextRow label="Notes">
            {noteParts.join(' · ')}
            {noteParts.length > 0 && track.spotify_id ? ' · ' : ''}
            <a
              href={`https://open.spotify.com/track/${track.spotify_id}`}
              target="_blank"
              rel="noreferrer"
              className="underline decoration-line underline-offset-2 hover:text-[var(--playlist-accent-strong)]"
            >
              Spotify ↗
            </a>
          </ContextRow>
        )}
      </div>

      {recentDuels.length > 0 && (
        <section className="mt-1 rounded-md border border-line bg-card px-2 py-1">
          <div className="flex items-baseline justify-between gap-2">
            <span className="font-mono text-[7px] font-semibold uppercase tracking-[0.1em] text-muted">
              Recent duels
            </span>
            <span className="font-mono text-[7px] uppercase tracking-[0.08em] text-muted">
              {preferredScope === 'personal' ? 'your picks' : 'the group'}
            </span>
          </div>
          <div className="mt-0.5 grid gap-x-4 sm:grid-cols-2">
            {recentDuels.map((duel) => (
              <DuelLine
                key={`${duel.opponent.id}-${duel.occurred_at}`}
                duel={duel}
              />
            ))}
          </div>
        </section>
      )}
    </div>
  )
}
