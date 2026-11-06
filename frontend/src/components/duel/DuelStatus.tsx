import { Kbd } from '../Kbd'

type DuelStatusProps = {
  round: number
  comparedPairs: number
  votesCast: number
  tracksSeen: number
  totalTracks: number
  undoing: boolean
  disabled: boolean
  onUndo: () => void
}

function UndoIcon() {
  return (
    <svg viewBox="0 0 20 20" className="size-3.5" aria-hidden="true">
      <path
        d="M7 6.5H4.5V4M4.8 6.2a6 6 0 1 1-1.2 4.6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

export function DuelStatus({
  round,
  comparedPairs,
  votesCast,
  tracksSeen,
  totalTracks,
  undoing,
  disabled,
  onUndo,
}: DuelStatusProps) {
  const progress =
    totalTracks > 0
      ? Math.min(100, Math.round((tracksSeen / totalTracks) * 100))
      : 0
  const canUndo = votesCast > 0 && !disabled && !undoing

  return (
    <section className="pv-duel-status" aria-label="Duel progress">
      <div className="flex items-center gap-2">
        <span className="pv-duel-round">Round {String(round).padStart(2, '0')}</span>

        <button
          type="button"
          onClick={onUndo}
          disabled={!canUndo}
          title="Undo last pick"
          aria-label="Undo last pick"
          className="pv-duel-undo ml-auto"
        >
          <UndoIcon />
          <span className="hidden sm:inline">{undoing ? 'Undoing…' : 'Undo'}</span>
          <span className="hidden lg:inline-flex">
            <Kbd>Z</Kbd>
          </span>
        </button>
      </div>

      <div className="mt-1.5 flex items-center justify-between gap-2 sm:mt-2">
        <span className="pv-duel-count">
          {comparedPairs.toLocaleString()} {comparedPairs === 1 ? 'duel' : 'duels'} ·{' '}
          {votesCast.toLocaleString()} {votesCast === 1 ? 'pick' : 'picks'}
        </span>
        <span className="pv-duel-coverage-label">{progress}% covered</span>
      </div>

      <div className="pv-duel-coverage">
        <div
          className="pv-duel-coverage-track"
          role="progressbar"
          aria-label="Songs covered"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={progress}
        >
          <div className="pv-duel-coverage-fill" style={{ width: `${progress}%` }} />
        </div>
      </div>

      <p className="pv-duel-hint pv-duel-status-hint mt-1.5 sm:mt-2 lg:hidden">
        Tap text · Swipe ← A/B → · Tap ▶ to listen
      </p>
    </section>
  )
}
