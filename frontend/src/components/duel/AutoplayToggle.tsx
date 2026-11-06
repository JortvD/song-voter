type AutoplayToggleProps = {
  enabled: boolean
  onToggle: () => void
  disabled?: boolean
}

function AutoplayIcon() {
  return (
    <svg viewBox="0 0 20 20" className="size-3" aria-hidden="true">
      <path d="M6.5 4.5l8 5.5-8 5.5z" fill="currentColor" />
    </svg>
  )
}

export function AutoplayToggle({
  enabled,
  onToggle,
  disabled = false,
}: AutoplayToggleProps) {
  return (
    <div className="flex justify-center">
      <button
        type="button"
        onClick={onToggle}
        disabled={disabled}
        aria-pressed={enabled}
        className={`pv-autoplay ${enabled ? 'is-on' : ''}`}
      >
        <span className="pv-autoplay-icon">
          <AutoplayIcon />
        </span>
        <span className="pv-autoplay-copy">
          <span className="pv-autoplay-label">Auto-preview</span>
          <span className="pv-autoplay-sub">
            {enabled ? 'Plays A, then B' : 'A → B'}
          </span>
        </span>
        <span className="pv-autoplay-switch" aria-hidden="true">
          <span />
        </span>
      </button>
    </div>
  )
}
