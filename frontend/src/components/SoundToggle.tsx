import { useState } from 'react'

import { isSoundEnabled, playStamp, setSoundEnabled } from '../lib/sound'

export function SoundToggle() {
  const [enabled, setEnabled] = useState(() => isSoundEnabled())

  function toggle() {
    const next = !enabled
    setEnabled(next)
    setSoundEnabled(next)

    if (next) {
      // Give the browser a user gesture to unlock audio and preview the stamp.
      playStamp()
    }
  }

  return (
    <button
      type="button"
      onClick={toggle}
      aria-pressed={enabled}
      className="inline-flex items-center gap-1.5 font-mono text-[9px] font-medium uppercase tracking-[0.16em] text-muted transition hover:text-ink"
      title={enabled ? 'Mute interface sounds' : 'Enable interface sounds'}
    >
      <svg viewBox="0 0 20 20" className="size-4" aria-hidden="true">
        <path
          d="M3 8v4h3l4 3V5L6 8H3z"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          strokeLinejoin="round"
        />
        {enabled ? (
          <>
            <path
              d="M13 7.5c1.1 1.4 1.1 3.6 0 5"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
            <path
              d="M15.5 5.5c2.1 2.5 2.1 6.5 0 9"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
            />
          </>
        ) : (
          <path
            d="M13 7l4 6M17 7l-4 6"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinecap="round"
          />
        )}
      </svg>
      Sound {enabled ? 'on' : 'off'}
    </button>
  )
}
