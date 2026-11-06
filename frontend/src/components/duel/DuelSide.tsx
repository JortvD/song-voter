import { useEffect, useState } from 'react'
import type { RefObject } from 'react'

import { Kbd } from '../Kbd'
import type { MatchupTrack } from '../../lib/types'

type DuelSideProps = {
  side: 'a' | 'b'
  track: MatchupTrack
  picked: boolean
  playing: boolean
  dimmed: boolean
  disabled: boolean
  audioRef: RefObject<HTMLAudioElement | null>
  onPlayStateChange: (playing: boolean) => void
  onTogglePreview: () => void
  onEnded: () => void
  onPick: () => void
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '0:00'

  const whole = Math.floor(seconds)
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`
}

function TransportIcon({ playing }: { playing: boolean }) {
  return playing ? (
    <svg viewBox="0 0 20 20" className="size-4" aria-hidden="true">
      <path
        d="M6.5 4.5v11M13.5 4.5v11"
        fill="none"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeLinecap="round"
      />
    </svg>
  ) : (
    <svg viewBox="0 0 20 20" className="size-4" aria-hidden="true">
      <path d="M7 4.5l8 5.5-8 5.5z" fill="currentColor" />
    </svg>
  )
}

function Equalizer() {
  return (
    <span className="pv-duel-eq" aria-hidden="true">
      <i />
      <i />
      <i />
    </span>
  )
}

export function DuelSide({
  side,
  track,
  picked,
  playing,
  dimmed,
  disabled,
  audioRef,
  onPlayStateChange,
  onTogglePreview,
  onEnded,
  onPick,
}: DuelSideProps) {
  const [position, setPosition] = useState(0)
  const [duration, setDuration] = useState(30)
  const previewAvailable = Boolean(track.preview_url)
  const label = side.toUpperCase()
  const percent = previewAvailable && duration > 0
    ? Math.min(100, (position / duration) * 100)
    : 0

  useEffect(() => {
    const audio = audioRef.current
    if (audio) {
      try {
        audio.pause()
        audio.currentTime = 0
      } catch {
        // Metadata may not be ready yet.
      }
    }

    setPosition(0)
    setDuration(30)
  }, [track.preview_url, audioRef])

  const coverContent = (
    <>
      {track.album_image_url ? (
        <img src={track.album_image_url} alt="" loading="lazy" draggable={false} />
      ) : (
        <span className="pv-duel-cover-fallback" aria-hidden="true">
          ♪
        </span>
      )}

      <span className="pv-duel-cover-badge" aria-hidden="true">
        {previewAvailable ? <TransportIcon playing={playing} /> : '↗'}
      </span>

      {previewAvailable ? (
        <span className="pv-duel-cover-progress" aria-hidden="true">
          <span style={{ width: `${percent}%` }} />
        </span>
      ) : (
        <span className="pv-duel-cover-empty" aria-hidden="true">
          Spotify
        </span>
      )}
    </>
  )

  return (
    <article
      className={`pv-duel-side pv-duel-side-${side} ${playing ? 'is-playing' : ''} ${
        picked ? 'is-picked' : ''
      } ${dimmed ? 'is-dimmed' : ''}`}
    >
      {previewAvailable ? (
        <button
          type="button"
          onClick={onTogglePreview}
          disabled={disabled}
          aria-label={
            playing
              ? `Pause 30 second preview of ${track.title}`
              : `Play 30 second preview of ${track.title}`
          }
          aria-pressed={playing}
          className={`pv-duel-cover ${playing ? 'is-playing' : ''}`}
        >
          {coverContent}
        </button>
      ) : (
        <a
          href={`https://open.spotify.com/track/${track.spotify_id}`}
          target="_blank"
          rel="noreferrer"
          aria-label={`Open ${track.title} in Spotify`}
          className="pv-duel-cover"
        >
          {coverContent}
        </a>
      )}

      <button
        type="button"
        onClick={onPick}
        disabled={disabled}
        aria-label={`Pick ${track.title} by ${track.artists}, side ${label}`}
        className="pv-duel-label"
      >
        <span className="pv-duel-label-top">
          <span className="pv-duel-side-tag">Side {label}</span>
          {playing && <Equalizer />}
        </span>

        <span className="pv-duel-title">{track.title}</span>
        <span className="pv-duel-artists">{track.artists}</span>
        {(track.release_year || track.album_type || track.primary_genre) && (
          <span className="pv-duel-meta">
            {[track.release_year, track.album_type, track.primary_genre]
              .filter(Boolean)
              .join(' · ')}
          </span>
        )}
      </button>

      <button
        type="button"
        onClick={onTogglePreview}
        disabled={disabled || !previewAvailable}
        aria-pressed={playing}
        aria-label={
          previewAvailable
            ? playing
              ? `Pause 30 second preview of ${track.title}`
              : `Play 30 second preview of ${track.title}`
            : `No preview available for ${track.title}`
        }
        className={`pv-duel-listen ${playing ? 'is-playing' : ''}`}
      >
        <span className="pv-duel-listen-icon">
          <TransportIcon playing={playing} />
        </span>
        <span className="pv-duel-listen-copy">
          <span className="pv-duel-listen-label">
            {previewAvailable ? (playing ? 'Pause' : 'Listen') : 'No preview'}
          </span>
          <span className="pv-duel-listen-time">
            {previewAvailable
              ? playing
                ? formatTime(position)
                : '30 sec'
              : 'Spotify'}
          </span>
        </span>
        {previewAvailable && (
          <span className="pv-duel-listen-progress" aria-hidden="true">
            <span style={{ width: `${percent}%` }} />
          </span>
        )}
      </button>

      <button
        type="button"
        onClick={onPick}
        disabled={disabled}
        className="pv-duel-desktop-pick"
      >
        <span aria-hidden="true">{side === 'a' ? '←' : '→'}</span>
        <span>Pick {label}</span>
        <span className="hidden sm:inline-flex">
          <Kbd>{side === 'a' ? '1' : '2'}</Kbd>
        </span>
      </button>

      {picked && (
        <span className="pv-duel-stamp" aria-hidden="true">
          Picked
        </span>
      )}

      {previewAvailable && (
        <audio
          ref={audioRef}
          src={track.preview_url ?? undefined}
          preload="metadata"
          className="hidden"
          aria-label={`30 second Spotify preview of ${track.title}`}
          onPlay={() => onPlayStateChange(true)}
          onPause={() => onPlayStateChange(false)}
          onEnded={(event) => {
            event.currentTarget.currentTime = 0
            setPosition(0)
            onPlayStateChange(false)
            onEnded()
          }}
          onTimeUpdate={(event) => setPosition(event.currentTarget.currentTime)}
          onLoadedMetadata={(event) => {
            const value = event.currentTarget.duration
            if (Number.isFinite(value) && value > 0) setDuration(value)
          }}
        />
      )}
    </article>
  )
}
