import { useEffect, useState } from 'react'
import type { ReactNode, RefObject } from 'react'

type PreviewPlayerProps = {
  previewUrl: string | null
  spotifyId: string
  title: string
  audioRef: RefObject<HTMLAudioElement | null>
  onPlayStateChange: (playing: boolean) => void
}

function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '0:00'

  const whole = Math.floor(seconds)
  return `${Math.floor(whole / 60)}:${String(whole % 60).padStart(2, '0')}`
}

function PlayIcon({ paused }: { paused: boolean }) {
  return paused ? (
    <svg viewBox="0 0 20 20" className="size-4" aria-hidden="true">
      <path d="M6.5 4.5l9 5.5-9 5.5z" fill="currentColor" />
    </svg>
  ) : (
    <svg viewBox="0 0 20 20" className="size-4" aria-hidden="true">
      <path d="M6.5 4.5v11M13.5 4.5v11" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  )
}

/**
 * A small native player for Spotify's 30-second preview MP3.
 *
 * We deliberately avoid the Spotify iframe: it injects a "Use the Spotify app"
 * prompt we cannot remove. Native previews keep the comparison flow clean, and
 * the parent can drive them from the A/B keyboard shortcuts.
 */
export function PreviewPlayer({
  previewUrl,
  spotifyId,
  title,
  audioRef,
  onPlayStateChange,
}: PreviewPlayerProps) {
  const [playing, setPlaying] = useState(false)
  const [position, setPosition] = useState(0)
  const [duration, setDuration] = useState(30)

  useEffect(() => {
    const audio = audioRef.current
    if (audio) {
      try {
        audio.pause()
        audio.currentTime = 0
      } catch {
        // The element may not have loaded metadata yet.
      }
    }
    setPlaying(false)
    setPosition(0)
    setDuration(30)

    return () => {
      try {
        audioRef.current?.pause()
      } catch {
        // Ignore teardown races.
      }
    }
  }, [previewUrl, audioRef])

  if (!previewUrl) {
    return (
      <div className="pv-preview">
        <div className="pv-preview-empty">
          <span className="font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
            No preview on this tape
          </span>
          <a
            href={`https://open.spotify.com/track/${spotifyId}`}
            target="_blank"
            rel="noreferrer"
            className="pv-link shrink-0 font-mono text-[9px] uppercase tracking-[0.12em]"
          >
            Open in Spotify
          </a>
        </div>
      </div>
    )
  }

  const total = duration > 0 ? duration : 30
  const progress = Math.min(100, (position / total) * 100)

  function toggle() {
    const audio = audioRef.current
    if (!audio) return

    if (audio.paused) {
      void audio.play().catch(() => onPlayStateChange(false))
    } else {
      audio.pause()
    }
  }

  return (
    <div className="pv-preview">
      <audio
        ref={audioRef}
        src={previewUrl}
        preload="metadata"
        aria-label={`30 second Spotify preview of ${title}`}
        onPlay={() => {
          setPlaying(true)
          onPlayStateChange(true)
        }}
        onPause={() => {
          setPlaying(false)
          onPlayStateChange(false)
        }}
        onEnded={(event) => {
          event.currentTarget.currentTime = 0
          setPlaying(false)
          setPosition(0)
          onPlayStateChange(false)
        }}
        onTimeUpdate={(event) => setPosition(event.currentTarget.currentTime)}
        onLoadedMetadata={(event) => {
          const value = event.currentTarget.duration
          if (Number.isFinite(value) && value > 0) setDuration(value)
        }}
      />

      <div className="flex items-center gap-2.5">
        <button
          type="button"
          onClick={toggle}
          aria-label={playing ? 'Pause preview' : 'Play preview'}
          aria-pressed={playing}
          className="pv-play-button"
        >
          <PlayIcon paused={!playing} />
        </button>

        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between font-mono text-[9px] uppercase tracking-[0.12em] text-muted">
            <span>30s preview</span>
            <span>
              {formatTime(position)} / {formatTime(total)}
            </span>
          </div>
          <div className="pv-preview-track">
            <div className="pv-preview-fill" style={{ width: `${progress}%` }} />
          </div>
        </div>
      </div>
    </div>
  )
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="pv-key">{children}</kbd>
}
