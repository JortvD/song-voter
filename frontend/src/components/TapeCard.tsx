import type { RefObject } from 'react'

import { Kbd, PreviewPlayer } from './PreviewPlayer'
import type { MatchupTrack } from '../lib/types'

type TapeCardProps = {
  side: 'a' | 'b'
  track: MatchupTrack
  picked: boolean
  playing: boolean
  disabled: boolean
  audioRef: RefObject<HTMLAudioElement | null>
  onPlayStateChange: (playing: boolean) => void
  onPick: () => void
}

export function TapeCard({
  side,
  track,
  picked,
  playing,
  disabled,
  audioRef,
  onPlayStateChange,
  onPick,
}: TapeCardProps) {
  return (
    <article
      className={`pv-tape-card ${playing ? 'is-playing' : ''} ${
        picked ? 'is-picked' : ''
      }`}
    >
      <div className="pv-tape-card-top">
        <span>Side {side.toUpperCase()}</span>
        <span className="flex items-center gap-2">
          {playing && (
            <span className="flex items-center gap-1 text-[9px]">
              <span className="size-1.5 animate-pulse rounded-full bg-[var(--playlist-accent)]" />
              playing
            </span>
          )}
          <span className="flex items-center gap-1">
            <Kbd>{side === 'a' ? '1' : '2'}</Kbd> pick
          </span>
        </span>
      </div>

      <div className="pv-tape-window">
        <div className="pv-tape-cover">
          {track.album_image_url ? (
            <img src={track.album_image_url} alt="" loading="lazy" />
          ) : (
            <span>♪</span>
          )}
        </div>

        <div className="pv-tape-meta min-w-0 flex-1">
          <h2>{track.title}</h2>
          <p className="mt-1.5 line-clamp-2 font-mono text-[9px] uppercase leading-relaxed tracking-[0.12em] text-muted">
            {track.artists}
          </p>
          {track.album && (
            <p className="mt-1 hidden truncate text-[11px] text-muted sm:block">
              {track.album}
            </p>
          )}
        </div>
      </div>

      <div className="pv-tape-reels" aria-hidden="true">
        <span className={`pv-reel ${playing ? 'pv-reel-spin' : ''}`} />
        <span className="pv-tape-ribbon" />
        <span className={`pv-reel ${playing ? 'pv-reel-spin' : ''}`} />
      </div>

      <PreviewPlayer
        previewUrl={track.preview_url}
        spotifyId={track.spotify_id}
        title={track.title}
        audioRef={audioRef}
        onPlayStateChange={onPlayStateChange}
      />

      <button
        type="button"
        onClick={onPick}
        disabled={disabled}
        className="pv-pick-button hidden md:inline-flex"
      >
        {picked ? 'Picked!' : `Pick ${side.toUpperCase()}`}
        <Kbd>{side === 'a' ? '1' : '2'}</Kbd>
      </button>

      {picked && <span className="pv-picked-stamp">Picked</span>}
    </article>
  )
}
