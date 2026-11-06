import type { StreakDay } from '../../lib/types'

type StreakDotsProps = {
  days: StreakDay[]
  showLabels?: boolean
}

function weekdayLabel(date: string) {
  try {
    return new Date(`${date}T12:00:00`).toLocaleDateString(undefined, {
      weekday: 'narrow',
    })
  } catch {
    return ''
  }
}

export function StreakDots({ days, showLabels = false }: StreakDotsProps) {
  return (
    <div
      className={`pv-streak-dots ${showLabels ? 'has-labels' : ''}`}
      aria-hidden="true"
    >
      {days.map((day) => (
        <span key={day.date} className="pv-streak-day">
          <span
            className={`pv-streak-dot ${day.met ? 'is-met' : ''} ${
              day.is_today ? 'is-today' : ''
            } ${day.picks > 0 && !day.met ? 'has-some' : ''}`}
          />
          {showLabels && (
            <span className="pv-streak-day-label">{weekdayLabel(day.date)}</span>
          )}
        </span>
      ))}
    </div>
  )
}
