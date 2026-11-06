import type { DuelHistoryItem, MatchupTrack } from '../../lib/types'

type HistoryDuelCardProps = {
  duel: DuelHistoryItem
  playingTrackId: string | null
  onTogglePreview: (track: MatchupTrack) => void
}

function formatDate(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return ''

  const diff = Date.now() - date.getTime()
  if (diff < 60_000) return 'Just now'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`
  if (diff < 604_800_000) return `${Math.floor(diff / 86_400_000)}d ago`

  return date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    year: date.getFullYear() === new Date().getFullYear() ? undefined : 'numeric',
  })
}

function PreviewIcon({ playing }: { playing: boolean }) {
  return playing ? (
    <svg viewBox="0 0 20 20" className="size-3.5" aria-hidden="true">
      <path
        d="M6.5 4.5v11M13.5 4.5v11"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
    </svg>
  ) : (
    <svg viewBox="0 0 20 20" className="size-3.5" aria-hidden="true">
      <path d="M7 4.5l8 5.5-8 5.5z" fill="currentColor" />
    </svg>
  )
}

function HistorySide({
  track,
  won,
  dimmed,
  playing,
  onTogglePreview,
}: {
  track: MatchupTrack
  won: boolean
  dimmed: boolean
  playing: boolean
  onTogglePreview: (track: MatchupTrack) => void
}) {
  const previewAvailable = Boolean(track.preview_url)

  const cover = (
    <>
      {track.album_image_url ? (
        <img src={track.album_image_url} alt="" loading="lazy" draggable={false} />
      ) : (
        <span aria-hidden="true">♪</span>
      )}

      <span className="pv-history-cover-action" aria-hidden="true">
        {previewAvailable ? <PreviewIcon playing={playing} /> : '↗'}
      </span>

      {won && (
        <span className="pv-history-check" aria-hidden="true">
          ✓
        </span>
      )}
    </>
  )

  return (
    <div
      className={`pv-history-side ${won ? 'is-winner' : ''} ${
        dimmed ? 'is-loser' : ''
      } ${playing ? 'is-playing' : ''}`}
    >
      {previewAvailable ? (
        <button
          type="button"
          onClick={() => onTogglePreview(track)}
          aria-pressed={playing}
          aria-label={
            playing
              ? `Pause preview of ${track.title}`
              : `Play preview of ${track.title}`
          }
          className="pv-history-cover"
        >
          {cover}
        </button>
      ) : (
        <a
          href={`https://open.spotify.com/track/${track.spotify_id}`}
          target="_blank"
          rel="noreferrer"
          aria-label={`Open ${track.title} in Spotify`}
          className="pv-history-cover"
        >
          {cover}
        </a>
      )}

      <div className="pv-history-copy">
        <p className="pv-history-title">{track.title}</p>
        <p className="pv-history-artist">{track.artists}</p>
      </div>
    </div>
  )
}

export function HistoryDuelCard({
  duel,
  playingTrackId,
  onTogglePreview,
}: HistoryDuelCardProps) {
  const picked = Boolean(duel.picked_track_id)
  const aWon = duel.picked_track_id === duel.track_a.id
  const bWon = duel.picked_track_id === duel.track_b.id
  const winnerTitle = aWon
    ? duel.track_a.title
    : bWon
      ? duel.track_b.title
      : null
  const loserTitle = aWon
    ? duel.track_b.title
    : bWon
      ? duel.track_a.title
      : null

  return (
    <article
      className="pv-history-card"
      aria-label={
        winnerTitle && loserTitle
          ? `Picked ${winnerTitle} over ${loserTitle}`
          : `No pick: ${duel.track_a.title} versus ${duel.track_b.title}`
      }
    >
      <div className="pv-history-card-top">
        <span className="pv-history-date">
          {formatDate(duel.picked_at ?? duel.seen_at)}
        </span>
        <span
          className={`pv-history-state ${
            picked ? 'is-picked' : 'is-skipped'
          }`}
        >
          {picked ? 'Picked' : 'No pick'}
        </span>
      </div>

      <div className="pv-history-sides">
        <HistorySide
          track={duel.track_a}
          won={aWon}
          dimmed={picked && !aWon}
          playing={playingTrackId === duel.track_a.id}
          onTogglePreview={onTogglePreview}
        />
        <HistorySide
          track={duel.track_b}
          won={bWon}
          dimmed={picked && !bWon}
          playing={playingTrackId === duel.track_b.id}
          onTogglePreview={onTogglePreview}
        />
      </div>
    </article>
  )
}
