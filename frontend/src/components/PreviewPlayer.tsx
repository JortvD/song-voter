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
      <div className="flex h-10 items-center justify-between gap-2 border border-line bg-card px-2.5">
        <span className="font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
          No preview available
        </span>
        <a
          href={`https://open.spotify.com/track/${spotifyId}`}
          target="_blank"
          rel="noreferrer"
          className="shrink-0 font-mono text-[8px] uppercase tracking-[0.12em] text-ink underline decoration-line underline-offset-2"
        >
          Open in Spotify
        </a>
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
    <div className="h-10 border border-line bg-card">
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
        onEnded={() => {
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

      <div className="flex h-full items-center gap-2.5 px-2.5">
        <button
          type="button"
          onClick={toggle}
          aria-label={playing ? 'Pause preview' : 'Play preview'}
          aria-pressed={playing}
          className="grid h-8 w-12 shrink-0 place-items-center border border-ink bg-ink font-mono text-[8px] uppercase tracking-[0.1em] text-paper transition hover:border-accent hover:bg-accent"
        >
          {playing ? 'Pause' : 'Play'}
        </button>

        <div className="min-w-0 flex-1">
          <div className="flex items-center justify-between font-mono text-[8px] uppercase tracking-[0.12em] text-muted">
            <span>30s Spotify preview</span>
            <span>
              {formatTime(position)} / {formatTime(total)}
            </span>
          </div>
          <div className="mt-1 h-0.5 bg-line">
            <div
              className="h-0.5 bg-ink transition-[width] duration-150"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  )
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <kbd className="inline-flex min-w-5 items-center justify-center border border-line bg-paper px-1 py-0.5 font-mono text-[9px] leading-none text-ink">
      {children}
    </kbd>
  )
}
