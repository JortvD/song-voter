import { Link } from 'react-router-dom'

import type { DuelHistoryItem, MatchupTrack } from '../../lib/types'

type RecentDuelsProps = {
  duels: DuelHistoryItem[]
  className?: string
  maxHeight?: number | null
}

function RecentSide({ track, won }: { track: MatchupTrack; won: boolean }) {
  return (
    <div className={`pv-recent-side ${won ? 'is-winner' : 'is-loser'}`}>
      <span className="pv-recent-cover">
        {track.album_image_url ? (
          <img src={track.album_image_url} alt="" loading="lazy" draggable={false} />
        ) : (
          <span aria-hidden="true">♪</span>
        )}
        {won && (
          <span className="pv-recent-check" aria-hidden="true">
            ✓
          </span>
        )}
      </span>

      <span className="pv-recent-copy">
        <span className="pv-recent-title">{track.title}</span>
        <span className="pv-recent-artist">{track.artists}</span>
      </span>
    </div>
  )
}

export function RecentDuels({
  duels,
  className = '',
  maxHeight,
}: RecentDuelsProps) {
  return (
    <section
      className={`pv-recent ${className}`}
      style={maxHeight != null ? { maxHeight: `${maxHeight}px` } : undefined}
      aria-label="Recent duels"
    >
      <div className="pv-recent-head">
        <span className="pv-recent-label">Recent duels</span>
        <Link to="/your-duels" className="pv-recent-link">
          See all →
        </Link>
      </div>

      <div className="pv-recent-list">
        {duels.length === 0 ? (
          <p className="pv-recent-empty">
            Your picked duels will collect here.
          </p>
        ) : (
          duels.map((duel) => {
            const winner = duel.picked_track_id
            const aWon = winner === duel.track_a.id
            const bWon = winner === duel.track_b.id
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
              <div
                key={`${duel.track_a.id}:${duel.track_b.id}`}
                className="pv-recent-row"
                role="group"
                aria-label={
                  winnerTitle && loserTitle
                    ? `Picked ${winnerTitle} over ${loserTitle}`
                    : 'Unpicked duel'
                }
              >
                <RecentSide track={duel.track_a} won={aWon} />
                <RecentSide track={duel.track_b} won={bWon} />
              </div>
            )
          })
        )}
      </div>

      {duels.length > 0 && <span className="pv-recent-fade" aria-hidden="true" />}
    </section>
  )
}
